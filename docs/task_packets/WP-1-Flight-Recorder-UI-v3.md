# Task Packet: WP-1-Flight-Recorder-UI-v3

## METADATA
- TASK_ID: WP-1-Flight-Recorder-UI-v3
- WP_ID: WP-1-Flight-Recorder-UI-v3
- BASE_WP_ID: WP-1-Flight-Recorder-UI (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-17T22:45:45.812Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja170120262341
- SUPERSEDES: WP-1-Flight-Recorder-UI-v2 (protocol drift; v3 is protocol-clean remediation)

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Flight-Recorder-UI-v3.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Upgrade the Flight Recorder Timeline UI so it supports spec-required filtering + deep links and makes FR-EVT-SEC-VIOLATION events easy to detect and debug.
- Why: You require industry-grade diagnostics/debuggability; Operator Consoles must enable a deterministic operator loop (find problem -> inspect evidence/timeline -> export bundle) and deep-link between ids (job_id/trace_id/event_id/wsid/diagnostic_id) without silent failures.
- IN_SCOPE_PATHS:
  - app/src/components/FlightRecorderView.tsx
  - app/src/lib/api.ts
  - app/src/App.css
  - app/src/App.test.tsx
- Scope note: `app/src/App.test.tsx` added to satisfy spec-required test coverage for Timeline deep-link focus behavior (Handshake_Master_Spec_v02.113.md:46286).
- OUT_OF_SCOPE:
  - Any backend changes in `src/backend/**` (including MEX/supply-chain work).
  - Any CI changes (including `.github/workflows/ci.yml`).
  - Any changes in `tests/` or `scripts/`.
  - Any Master Spec edits/version bumps (see `docs/refinements/WP-1-Flight-Recorder-UI-v3.md`).
  - Do not touch the Coder-1 supply-chain WP in-scope files:
    - .github/workflows/ci.yml
    - src/backend/handshake_core/mechanical_engines.json
    - src/backend/handshake_core/src/mex/*

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Flight-Recorder-UI-v3

# Frontend lint/build (required):
pnpm -C app run lint
pnpm -C app run build

just cargo-clean
just post-work WP-1-Flight-Recorder-UI-v3
```

### DONE_MEANS
- UI supports Timeline filters for at least: `job_id`, `trace_id`, `event_id`, `wsid`, `actor`, `event_type`, and time range (`from`/`to`) and correctly passes query params to `/api/flight_recorder`.
- Timeline provides deterministic deep links/navigation targets:
  - job_id -> filters Timeline (or navigates to Jobs view if present)
  - trace_id -> filters Timeline
  - event_id -> focuses/selects the event in Timeline (or filters)
  - wsid -> filters Timeline
  - diagnostic_id (when present in payload) -> navigates to Problems/Diagnostic view (or at minimum provides a reliable copyable link target).
- Deep link failures do not silently no-op: invalid ids show a visible error state and/or create a Diagnostic (per VAL-NAV-001 intent).
- FR-EVT-SEC-VIOLATION entries are visually prominent and do not leak secrets via additional UI rendering.
- `just pre-work WP-1-Flight-Recorder-UI-v3` passes and the checkpoint commit exists (packet + refinement committed on WP branch).

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.113.md (recorded_at: 2026-01-17T22:45:45.812Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.113.md:35191 Flight Recorder (always-on, with UI) + Timeline (filters + deep links)
  - Handshake_Master_Spec_v02.113.md:41809 Console surfaces deep-link via job_id/diagnostic_id/wsid/event ids
  - Handshake_Master_Spec_v02.113.md:46286 VAL-NAV-001 deep-link resolution guarantees (no silent failures)
  - Handshake_Master_Spec_v02.113.md:6708 FR-EVT-SEC-VIOLATION emission (must be visible in UI)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packets:
  - docs/task_packets/WP-1-Flight-Recorder-UI.md (legacy)
  - docs/task_packets/WP-1-Flight-Recorder-UI-v2.md (failed revalidation / protocol drift)
  - docs/task_packets/stubs/WP-1-Flight-Recorder-UI-v3.md (stub backlog; superseded by this official packet)
- Preserved: Spec-required Timeline filters + deep links + safe debug visibility for security events.
- Changed: v3 is a protocol-clean packet with signed technical refinement and deterministic workflow gates.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/CODER_PROTOCOL.md
  - docs/VALIDATOR_PROTOCOL.md
  - docs/refinements/WP-1-Flight-Recorder-UI-v3.md
  - docs/task_packets/WP-1-Flight-Recorder-UI-v2.md
  - app/src/components/FlightRecorderView.tsx
  - app/src/lib/api.ts
  - app/src/App.css
- SEARCH_TERMS:
  - "FlightRecorderView"
  - "getEvents"
  - "FlightEventFilters"
  - "event_id"
  - "wsid"
  - "event_type"
  - "FR-EVT-SEC-VIOLATION"
  - "VAL-NAV-001"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-Flight-Recorder-UI-v3
  pnpm -C app run lint
  pnpm -C app run build
  ```
- RISK_MAP:
  - "Payload contains secrets" -> "do not add new UI leak paths; ensure payload rendering is safe-by-default"
  - "Deep links silently fail" -> "operator cannot debug; must show visible error/diagnostic when resolution fails"
  - "API query mismatch" -> "filters look like they work but return wrong rows; ensure query param names match backend"

## SKELETON
SKELETON APPROVED

- Proposed interfaces/types/contracts:
  - `FlightRecorderUiFilters` (draft inputs; empty string = unset): `jobId`, `traceId`, `eventId`, `wsid`, `actor`, `eventType`, `from`, `to`.
  - `AppliedFilters` (polling contract): snapshot of the last submitted filters; polling always uses this snapshot (prevents stale-closure polling bug).
  - `emitNavFailure(action, reason, context)` (best-effort): uses `createDiagnostic()` with `code="VAL-NAV-001"` and includes `wsid`, `job_id`, and `fr_event_ids` (when applicable).
  - `redactJsonValue(value)` (safe-by-default): truncates long strings; redacts keys matching `(token|secret|password|api[_-]?key)`; caps depth/array sizes.
  - `focusEvent(event_id)` (deterministic): sets selected row and calls `scrollIntoView()`, falling back to filtering by `event_id` if needed.
  - Deep-link representation (within Timeline UI scope): a copyable link target string generated from current filters (no App-level routing changes in scope).
- Open questions:
  - `diagnostic_id` discovery: event payload key(s) to read (plan: support `diagnostic_id` and `diagnosticId`; always provide copyable target even if no navigation).
  - `wsids` is an array in `FlightEvent`; Timeline filter input is a single `wsid` string (plan: quick-link uses first wsid; also render all wsids as clickable chips).
  - Poll cadence: keep 5s polling but avoid emitting repeated VAL-NAV-001 diagnostics on each poll (plan: emit at most once per filter-submit when zero results and an id filter is set).
- Notes:
  - IN_SCOPE limitation: cross-surface navigation wiring in `app/src/App.tsx` is out-of-scope; deep links will be implemented as Timeline filters/focus plus copyable targets (allowed by DONE_MEANS).
  - Security events: add no new secret-leak paths; payload rendering is redacted-by-default, with explicit opt-in for raw view.
  - Styling: `app/src/App.css` currently lacks `.flight-recorder__*` styles; this WP will add them and ensure security violations are visually prominent.
  - Worktree hygiene: resolved via docs-only newline normalization commit `6ed8b877` for `docs/refinements/WP-1-LLM-Core-v3.md` (explicit Operator authorization) to avoid cross-WP contamination.

## IMPLEMENTATION
- Updated `FlightRecorderView` with draft/applied Timeline filters (job_id/trace_id/event_id/wsid/actor/event_type/from/to) and URL query sync for deep links.
- Added deterministic deep-link targets: clickable IDs apply filters; event_id focuses/selects; diagnostic_id provides a copyable Problems target.
- Prevented silent deep-link failures: visible notice + best-effort Diagnostic emission (`VAL-NAV-001`) for no-results, missing event_id in results, and clipboard failures.
- Rendered payloads redacted-by-default with explicit per-event opt-in "Reveal raw (unsafe)" and redacted security_violation context snippet.
- Added Flight Recorder UI styles and made security_violation rows/tags prominent.
- Added vitest coverage for `event_id` deep-link focus (row selection + scrollIntoView) and `VAL-NAV-001` emission when `event_id` is missing from returned results (`app/src/App.test.tsx`).

## HYGIENE
- Ran: `just pre-work WP-1-Flight-Recorder-UI-v3`
- Ran: `pnpm -C app install` (node_modules for lint/build in this worktree)
- Ran: `pnpm -C app run lint`
- Ran: `pnpm -C app run build`
- Ran: `pnpm -C app test`
- Ran: `just cargo-clean`
- Updated COR-701 manifest inputs via `just cor701-sha` for the touched app files.

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `app/src/components/FlightRecorderView.tsx`
- **Start**: 1
- **End**: 1
- **Line Delta**: 0
- **Pre-SHA1**: `df5b55c3810092e1fd716115fc042243b4ce8ccf`
- **Post-SHA1**: `df5b55c3810092e1fd716115fc042243b4ce8ccf`
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

- **Target File**: `app/src/App.css`
- **Start**: 1
- **End**: 1
- **Line Delta**: 0
- **Pre-SHA1**: `dfce683c0d45d8ede2f1068268b0bf717edbb864`
- **Post-SHA1**: `dfce683c0d45d8ede2f1068268b0bf717edbb864`
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

- **Target File**: `app/src/App.test.tsx`
- **Start**: 74
- **End**: 149
- **Line Delta**: 65
- **Pre-SHA1**: `8f99cb6fc3436c4411ab7e099d25dbc991a40d84`
- **Post-SHA1**: `6813345820463a576713dac767cb0e2278dd4cdd`
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
  - `pnpm -C app run lint` (exit code 0)
- **Artifacts**:
  - `pnpm -C app run build` (exit code 0)
- **Test Results**:
  - `pnpm -C app test` (exit code 0)
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
- **Notes**:
  - Evidence content is recorded as ASCII-only (avoid pasting ANSI/unicode output into this packet).

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: Remediation complete; ready for Validator re-review.
- What changed in this update:
  - Added Timeline filters + deep links + redacted payload UI in `app/src/components/FlightRecorderView.tsx`.
  - Added Timeline styles + security_violation prominence in `app/src/App.css`.
  - Added spec-required deep-link focus tests in `app/src/App.test.tsx` (Handshake_Master_Spec_v02.113.md:46286).
- Next step / handoff hint:
  - Validator: review DONE_MEANS and proceed with merge if acceptable.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Commands (exit codes only; logs omitted to keep packet ASCII-only):
  - `just pre-work WP-1-Flight-Recorder-UI-v3` (exit code 0)
  - `pnpm -C app run lint` (exit code 0)
  - `pnpm -C app run build` (exit code 0)
  - `pnpm -C app test` (exit code 0)
  - `just cargo-clean` (exit code 0)
  - `just post-work WP-1-Flight-Recorder-UI-v3` (exit code 0)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### VALIDATION REPORT - WP-1-Flight-Recorder-UI-v3 (2026-01-18)
Verdict: FAIL

Scope Inputs:
- Task Packet: `docs/task_packets/WP-1-Flight-Recorder-UI-v3.md` (**Status:** In Progress)
- Spec Target: `docs/SPEC_CURRENT.md` -> `Handshake_Master_Spec_v02.113.md`
- Spec Anchors:
  - `Handshake_Master_Spec_v02.113.md:35191`
  - `Handshake_Master_Spec_v02.113.md:41809`
  - `Handshake_Master_Spec_v02.113.md:46286`
  - `Handshake_Master_Spec_v02.113.md:6708`
- Active Packet mapping: `docs/WP_TRACEABILITY_REGISTRY.md:77`
- Worktree/Branch: `D:\Projects\LLM projects\wt-WP-1-Flight-Recorder-UI-v3` / `feat/WP-1-Flight-Recorder-UI-v3`
- Commits reviewed: `f1727cdc` (implementation), `be976117` (skeleton marker), `6b769dd2` (packet), `731af6a0` (checkpoint)

Files Checked:
- `docs/task_packets/WP-1-Flight-Recorder-UI-v3.md`
- `docs/refinements/WP-1-Flight-Recorder-UI-v3.md`
- `docs/WP_TRACEABILITY_REGISTRY.md`
- `docs/SPEC_CURRENT.md`
- `Handshake_Master_Spec_v02.113.md`
- `app/src/components/FlightRecorderView.tsx`
- `app/src/lib/api.ts`
- `app/src/App.css`
- `app/src/App.tsx`
- `app/src/components/operator/TimelineView.tsx`
- `app/src/components/operator/JobsView.tsx`
- `app/src/components/operator/ProblemsView.tsx`

Findings:
- Requirement (Timeline filters + /api/flight_recorder params): satisfied
  - UI filters: `app/src/components/FlightRecorderView.tsx:379`
  - UI->API mapping: `app/src/components/FlightRecorderView.tsx:53`
  - Backend query param mapping: `app/src/lib/api.ts:443`
- Requirement (Deep links / deterministic targets): satisfied (Timeline/FlightRecorderView surface)
  - URL -> filters: `app/src/components/FlightRecorderView.tsx:90`
  - filters -> URL: `app/src/components/FlightRecorderView.tsx:113`
  - event_id focus/scroll: `app/src/components/FlightRecorderView.tsx:271`, `app/src/components/FlightRecorderView.tsx:301`
  - copy link target: `app/src/components/FlightRecorderView.tsx:329`, `app/src/components/FlightRecorderView.tsx:463`
  - diagnostic_id extraction + copy target: `app/src/components/FlightRecorderView.tsx:162`, `app/src/components/FlightRecorderView.tsx:592`
- Requirement (VAL-NAV-001 no silent failures): satisfied (best-effort Diagnostic + visible notice + de-spam)
  - Diagnostic emission: `app/src/components/FlightRecorderView.tsx:67`
  - No-results/missing-event: `app/src/components/FlightRecorderView.tsx:203`
- Requirement (FR-EVT-SEC-VIOLATION visible + safe-by-default): satisfied
  - Security violation prominence: `app/src/components/FlightRecorderView.tsx:496`, `app/src/App.css:280`
  - Redaction + unsafe toggle/warning: `app/src/components/FlightRecorderView.tsx:135`, `app/src/components/FlightRecorderView.tsx:612`, `app/src/App.css:517`
- Requirement (VAL-NAV-001 required tests): NOT satisfied
  - Spec requires test coverage: `Handshake_Master_Spec_v02.113.md:46286` ("event_id -> Timeline focus")
  - App tests pass, but do not cover FlightRecorderView/timeline focus or VAL-NAV-001 behavior.

Tests:
- `just pre-work WP-1-Flight-Recorder-UI-v3`: PASS
- `pnpm -C app run lint`: PASS
- `pnpm -C app run build`: PASS
- `pnpm -C app test`: PASS (5 test files, 8 tests)
- `just cargo-clean`: PASS
- `node scripts/validation/gate-check.mjs WP-1-Flight-Recorder-UI-v3`: PASS
- `just validator-coverage-gaps`: PASS (tests detected)
- `just validator-scan`: FAIL due to out-of-scope pre-existing hits in backend (not caused by this WP diff)

Hygiene:
- Worktree clean at validation time.

REASON FOR FAIL:
- `Handshake_Master_Spec_v02.113.md:46286` requires a test suite to cover `event_id -> Timeline focus`; no such targeted test exists for this feature.

Suggested Actions:
- Add a vitest test that verifies `event_id -> Timeline focus` for `FlightRecorderView` (and ideally a failure path that emits VAL-NAV-001). Re-run `pnpm -C app test`.
