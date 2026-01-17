# Task Packet: WP-1-Flight-Recorder-UI-v3

## METADATA
- TASK_ID: WP-1-Flight-Recorder-UI-v3
- WP_ID: WP-1-Flight-Recorder-UI-v3
- BASE_WP_ID: WP-1-Flight-Recorder-UI (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-17T22:45:45.812Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator-codex-cli
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.113.md
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
