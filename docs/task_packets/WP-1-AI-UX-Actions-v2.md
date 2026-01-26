# Task Packet: WP-1-AI-UX-Actions-v2

## METADATA
- TASK_ID: WP-1-AI-UX-Actions-v2
- WP_ID: WP-1-AI-UX-Actions-v2
- BASE_WP_ID: WP-1-AI-UX-Actions
- DATE: 2026-01-26T00:01:32.686Z
- REQUESTOR: User
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- CODER_MODEL: <unclaimed> (assigned: Coder-B)
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** In Progress
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja260120260054

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-AI-UX-Actions-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Add a Command Palette-style UI entrypoint for explicit AI actions in the Document editor, starting with `doc_summarize` ("Summarize document") implemented via the existing `POST /api/jobs` contract (through `app/src/lib/api.ts`), and surface the resulting job state/output in the UI.
- Why: Establish a stable, model-agnostic UX surface for invoking AI jobs (explicit actions) without coupling to the upcoming local/cloud/tool orchestration refactor; enables fast iteration on AI UX while preserving capability/logging/trace correlation paths.
- IN_SCOPE_PATHS:
  - app/src/components/DocumentView.tsx
  - app/src/components/TiptapEditor.tsx
  - app/src/components/JobResultPanel.tsx
  - app/src/lib/api.ts
  - app/src/components/DocumentView.test.tsx (update/add as needed)
  - app/src/components/CommandPalette*.tsx (new; name may vary)
  - app/src/App.css (if styling needed)
- OUT_OF_SCOPE:
  - Any backend changes (do not modify `src/backend/**` in this WP).
  - Any changes to AI-Ready Data Architecture (WP-1-AI-Ready-Data-Architecture-v1 scope).
  - "Ask about this document" and "Rewrite selection" semantics if backend job_inputs/contracts are not implemented yet (capture as follow-up WP(s) after spec refactor).
  - Any local vs cloud model routing changes (runtime policy/refactor planned separately).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-AI-UX-Actions-v2

# Frontend checks:
pnpm -C app run lint
pnpm -C app test

# MEDIUM risk expectation: run full hygiene if feasible (or record why not in the packet):
just validate

# Optional hygiene:
just cargo-clean

# Manual smoke (run in a separate terminal):
# pnpm -C app tauri dev
# - Open a document, open Command Palette, run "Summarize document"
# - Verify job runs and output is visible

just post-work WP-1-AI-UX-Actions-v2
```

### DONE_MEANS
- A Command Palette UI can be opened from the Document editor surface (keyboard shortcut and/or button), and it lists "Summarize document" as an action.
- Triggering "Summarize document" creates an AI job using `job_kind="doc_summarize"` and the existing protocol id used by the app (do not introduce new protocol ids in this WP), targeting the current document id.
- The UI surfaces job progress and displays the job output via `JobResultPanel` (or a successor) once completed, and handles error states without crashing.
- No direct `fetch` is introduced in components; API calls route through `app/src/lib/api.ts` helpers.
- `just pre-work WP-1-AI-UX-Actions-v2` and `just post-work WP-1-AI-UX-Actions-v2` pass on the WP branch.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.117.md (recorded_at: 2026-01-26T00:01:32.686Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR: Handshake_Master_Spec_v02.117.md 2.5.10.3.1; 2.6.6.2.8.1; 11.5.2
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior packets (superseded / history):
  - docs/task_packets/WP-1-AI-UX-Actions.md
- Preserved intent:
  - Provide an explicit editor entrypoint for AI actions via the Command Palette pattern.
- Changes in this v2 remediation:
  - Re-anchored to Master Spec v02.117 Main Body anchors (not Roadmap-only).
  - Tightened scope to frontend-only wiring (no backend changes) to avoid cross-WP merge friction during parallel Phase 1 work.
  - Explicitly defers "Ask about this document" and "Rewrite selection" semantics until backend job_inputs/contracts are implemented and spec refactor is complete.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.117.md
  - docs/refinements/WP-1-AI-UX-Actions-v2.md
  - docs/task_packets/WP-1-AI-UX-Actions-v2.md
  - app/src/components/DocumentView.tsx
  - app/src/components/TiptapEditor.tsx
  - app/src/components/JobResultPanel.tsx
  - app/src/lib/api.ts
- SEARCH_TERMS:
  - "createJob("
  - "doc_summarize"
  - "protocol_id"
  - "JobResultPanel"
  - "tiptap"
- RUN_COMMANDS:
  ```bash
  just pre-work WP-1-AI-UX-Actions-v2
  pnpm -C app run lint
  pnpm -C app test
  just post-work WP-1-AI-UX-Actions-v2
  ```
- RISK_MAP:
  - "wrong doc target" -> "job created for wrong document id; results confusing/wrong"
  - "stale UI state" -> "palette/job panel does not reset on doc switch; leaks old job output"
  - "UX regression" -> "new keyboard shortcut conflicts with editor behavior; breaks typing/selection"

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
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.117.md
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
