# Task Packet: WP-1-AI-UX-Summarize-Display-v2

## METADATA
- TASK_ID: WP-1-AI-UX-Summarize-Display-v2
- WP_ID: WP-1-AI-UX-Summarize-Display-v2
- BASE_WP_ID: WP-1-AI-UX-Summarize-Display (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-01T14:24:17.979Z
- REQUESTOR: ilja (Operator)
- AGENT_ID: user_orchestrator (Codex CLI)
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja010220261515
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-AI-UX-Summarize-Display-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Make the AI Jobs "Job Inspector -> Summary" view leak-aware: default to hash-based IO display, and require explicit reveal for any output preview.
- Why: The spec requires a Job Inspector with hash-based IO handling; summaries can contain sensitive content and must not be auto-rendered.
- IN_SCOPE_PATHS:
  - docs/task_packets/WP-1-AI-UX-Summarize-Display-v2.md
  - app/src/components/AiJobsDrawer.tsx
  - app/src/components/JobResultPanel.tsx (or renamed Job Inspector component)
  - app/src/state/aiJobs.ts
  - app/src/lib/api.ts (hash helpers; if needed)
  - app/src/components/DocumentView.tsx (only if needed for parity with Jobs Drawer)
- OUT_OF_SCOPE:
  - Backend API changes unless strictly required (prefer using existing AiJob fields)
  - Writing summaries back into document blocks (persisted DerivedContent) as part of this WP
  - Introducing a new Jobs UI surface beyond the existing AI Jobs drawer

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-AI-UX-Summarize-Display-v2
cd app
pnpm run lint
pnpm test
just cargo-clean
just post-work WP-1-AI-UX-Summarize-Display-v2
```

### DONE_MEANS
- AI Jobs Drawer "detail" view shows job_id + trace_id and a safe Summary view (plain text).
- Summary view defaults to showing outputs_hash (computed deterministically from job_outputs when present) without auto-rendering raw outputs.
- A user action is required to reveal any outputs preview, and preview is disabled by default in safety-sensitive contexts.
- Unit tests cover completed vs running vs failed jobs and ensure outputs are not auto-rendered.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.123.md (recorded_at: 2026-02-01T14:24:17.979Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 10.5.5.2 (Jobs surface - Job Inspector tabs) and 10.5.6.5.3 (BundleJob jobs.json/job.json - hash-based IO surfacing)
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packet: docs/task_packets/WP-1-AI-UX-Summarize-Display.md (superseded; FAIL; stale spec references and incomplete governance fields)
- Preserved: show doc_summarize results in UI and handle queued/running/completed/failed states.
- Changed: pinned spec baseline v02.123; aligned to Operator Consoles Job Inspector requirement; adopted hash-first IO display with explicit reveal for previews.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.123.md (10.5.5.2 Jobs + 10.5.6.5.3 BundleJob)
  - app/src/components/AiJobsDrawer.tsx
  - app/src/components/JobResultPanel.tsx
  - app/src/state/aiJobs.ts
  - app/src/lib/api.ts (AiJob type, sha256 helpers)
- SEARCH_TERMS:
  - "job_outputs"
  - "safety_mode"
  - "access_mode"
  - "sha256HexUtf8"
  - "AiJobsDrawer"
- RUN_COMMANDS:
  ```bash
  pnpm -C app test
  pnpm -C app run lint
  ```
- RISK_MAP:
  - "leak raw output in UI" -> "hash-first + explicit reveal; plain text only"
  - "XSS via outputs" -> "render as text/pre only; no HTML"
  - "stale job selection" -> "bind display to selected job_id + show outputs_hash"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: server->ui (AiJob.job_outputs -> UI summary display)
- SERVER_SOURCES_OF_TRUTH:
  - job_id + trace_id returned by backend
- REQUIRED_PROVENANCE_FIELDS:
  - job_id
  - trace_id
- VERIFICATION_PLAN:
  - UI shows job_id + trace_id; tests assert outputs are not auto-rendered
- ERROR_TAXONOMY_PLAN:
  - fetch_error (job retrieval failed)
  - parse_error (outputs not JSON-parseable)
- UI_GUARDRAILS:
  - default collapsed outputs; explicit reveal
  - never render outputs as HTML
- VALIDATOR_ASSERTIONS:
  - hash-first summary display aligned to spec anchors
  - no automatic rendering of raw outputs

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
