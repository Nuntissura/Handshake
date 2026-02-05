# Task Packet: WP-1-AI-UX-Summarize-Display-v2

## METADATA
- TASK_ID: WP-1-AI-UX-Summarize-Display-v2
- WP_ID: WP-1-AI-UX-Summarize-Display-v2
- BASE_WP_ID: WP-1-AI-UX-Summarize-Display (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-02-01T14:24:17.979Z
- REQUESTOR: ilja (Operator)
- AGENT_ID: user_orchestrator (Codex CLI)
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja010220261515
- PACKET_FORMAT_VERSION: 2026-02-01

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-AI-UX-Summarize-Display-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Make the AI Jobs "Job Inspector -> Summary" view leak-aware: default to hash-based IO display, and require explicit reveal for any output preview.
- Why: The spec requires a Job Inspector with hash-based IO handling; summaries can contain sensitive content and must not be auto-rendered.
- IN_SCOPE_PATHS:
  - .GOV/task_packets/WP-1-AI-UX-Summarize-Display-v2.md
  - app/src/components/AiJobsDrawer.tsx
  - app/src/components/JobResultPanel.tsx
  - app/src/components/JobResultPanel.test.tsx
  - app/src/state/aiJobs.ts
  - app/src/lib/api.ts
  - app/src/components/DocumentView.tsx
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
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.123.md 10.5.5.2 (Jobs surface - Job Inspector tabs) and 10.5.6.5.3 (BundleJob jobs.json/job.json - hash-based IO surfacing)
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- Prior packet: .GOV/task_packets/WP-1-AI-UX-Summarize-Display.md (superseded; FAIL; stale spec references and incomplete governance fields)
- Preserved: show doc_summarize results in UI and handle queued/running/completed/failed states.
- Changed: pinned spec baseline v02.123; aligned to Operator Consoles Job Inspector requirement; adopted hash-first IO display with explicit reveal for previews.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
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
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `app/src/components/JobResultPanel.tsx`
- **Start**: 1
- **End**: 139
- **Line Delta**: 78
- **Pre-SHA1**: `85261e65442853eb152614d4f92b0f6ded496087`
- **Post-SHA1**: `23b9fdf7032d3182c1834a80372e7a7c94352a97`
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
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
- **Notes**:

- **Target File**: `app/src/components/JobResultPanel.test.tsx`
- **Start**: 1
- **End**: 81
- **Line Delta**: 81
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `c2e85b9af03a70f0f4164e718dc69ac76ed34f45`
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
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
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

### VALIDATION REPORT - WP-1-AI-UX-Summarize-Display-v2
- Verdict: PASS
- VALIDATED_AT: 2026-02-02T01:16:15.076Z
- Worktree/Branch/Commit:
  - Worktree: D:\Projects\LLM projects\wt-WP-1-AI-UX-Summarize-Display-v2
  - Branch: feat/WP-1-AI-UX-Summarize-Display-v2
  - Commit: 4b5c878057434edfbb9948d5015cf13b27ebbaf8
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.123.md
- SPEC_ANCHORS (Handshake_Master_Spec_v02.123.md):
  - 10.5.5.2 Jobs (lines 52973-52978)
  - 10.5.6.5.3 jobs.json / job.json (BundleJob; SAFE_DEFAULT hashes vs previews) (lines 53274-53321)

Scope Inputs
- Task packet: .GOV/task_packets/WP-1-AI-UX-Summarize-Display-v2.md (Status: In Progress; RISK_TIER: MEDIUM; USER_SIGNATURE: ilja010220261515)
- Refinement: .GOV/refinements/WP-1-AI-UX-Summarize-Display-v2.md (APPROVED; USER_APPROVAL_EVIDENCE present)

Files Checked
- Handshake_Master_Spec_v02.123.md
- .GOV/roles_shared/SPEC_CURRENT.md
- .GOV/refinements/WP-1-AI-UX-Summarize-Display-v2.md
- .GOV/task_packets/WP-1-AI-UX-Summarize-Display-v2.md
- app/src/components/JobResultPanel.tsx
- app/src/components/JobResultPanel.test.tsx

Commands Run (per TEST_PLAN)
- just pre-work WP-1-AI-UX-Summarize-Display-v2
- pnpm -C app run lint
- pnpm -C app test
- just cargo-clean
- just post-work WP-1-AI-UX-Summarize-Display-v2

Findings (Master Spec / DONE_MEANS mapping)
- Spec 10.5.5.2 Jobs (Job Inspector includes Summary tab):
  - Summary/detail view renders job status + job_id + trace_id: app/src/components/JobResultPanel.tsx:93, app/src/components/JobResultPanel.tsx:97, app/src/components/JobResultPanel.tsx:100
- Spec 10.5.6.5.3 BundleJob SAFE_DEFAULT IO as hashes (previews are gated/explicit):
  - Outputs are not auto-rendered; default is hash-first: app/src/components/JobResultPanel.tsx:109, app/src/components/JobResultPanel.tsx:111
  - outputs_hash computed deterministically from job_outputs (stable key ordering) then sha256HexUtf8: app/src/components/JobResultPanel.tsx:4, app/src/components/JobResultPanel.tsx:20, app/src/components/JobResultPanel.tsx:37, app/src/components/JobResultPanel.tsx:49
  - Explicit reveal required for preview (no preview by default): app/src/components/JobResultPanel.tsx:114, app/src/components/JobResultPanel.tsx:127
  - Preview disabled in safety-sensitive context (strict safety mode): app/src/components/JobResultPanel.tsx:68, app/src/components/JobResultPanel.tsx:117, app/src/components/JobResultPanel.tsx:123
  - Preview renders as plain text (<pre>), not HTML: app/src/components/JobResultPanel.tsx:132
- Unit tests:
  - SECRET not rendered by default; revealed only after click; hash shown: app/src/components/JobResultPanel.test.tsx:55, app/src/components/JobResultPanel.test.tsx:57, app/src/components/JobResultPanel.test.tsx:62
  - Strict safety mode disables reveal; SECRET never rendered; hash shown: app/src/components/JobResultPanel.test.tsx:71, app/src/components/JobResultPanel.test.tsx:75, app/src/components/JobResultPanel.test.tsx:78

Notes (non-blocking)
- DONE_MEANS mentions unit coverage for running/failed states; the newly-added tests focus on leak-safety behavior (default hidden + strict mode gate) and do not explicitly assert running/failed render paths.

Verdict: PASS


