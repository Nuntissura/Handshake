# Task Packet: WP-1-AI-UX-Actions-v2

## METADATA
- TASK_ID: WP-1-AI-UX-Actions-v2
- WP_ID: WP-1-AI-UX-Actions-v2
- BASE_WP_ID: WP-1-AI-UX-Actions
- DATE: 2026-01-26T00:01:32.686Z
- REQUESTOR: User
- AGENT_ID: codex-cli
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja260120260054

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-AI-UX-Actions-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Add a Command Palette-style UI entrypoint for explicit AI actions in the Document editor, starting with `doc_summarize` ("Summarize document") implemented via the existing `POST /api/jobs` contract (through `app/src/lib/api.ts`), with job_inputs that include a `DocsAiJobProfile`, and surface the resulting job state/output in a global job tracker UI.
- Why: Establish a stable, model-agnostic UX surface for invoking AI jobs (explicit actions) without coupling to the upcoming local/cloud/tool orchestration refactor; enables fast iteration on AI UX while preserving capability/logging/trace correlation paths.
- IN_SCOPE_PATHS:
  - app/src/App.tsx
  - app/src/App.css
  - app/src/lib/api.ts
  - app/src/state/aiJobs.ts
  - app/src/components/AiJobsDrawer.tsx
  - app/src/components/CommandPalette.tsx
  - app/src/components/DocumentView.tsx
  - app/src/components/DocumentView.test.tsx
  - app/src/components/JobResultPanel.tsx
  - app/src/components/TiptapEditor.tsx
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
- A Command Palette UI can be opened from the Document editor surface (button + `Ctrl/Cmd+K` primary and `Ctrl/Cmd+Shift+P` fallback), and it lists "Summarize document" as an action.
- Triggering "Summarize document" creates an AI job using `job_kind="doc_summarize"` and the existing protocol id used by the app (do not introduce new protocol ids in this WP), targeting the current document id and sending `job_inputs` that include a valid `DocsAiJobProfile` per Master Spec 2.6.6.6.4 (min: `{ doc_id, selection: null, layer_scope: "Document" }`).
- The app provides a global AI Jobs tracker UI (drawer/tray) that persists across document switches and app reloads (rehydrates from `localStorage`) and displays queued/running/completed/failed jobs; selecting a job shows output via `JobResultPanel` (or a successor) and handles error states without crashing.
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
- `app/src/state/aiJobs.ts` (new; name may vary): global in-memory job tracker store.
  - Data model (persisted): `{ jobId, jobKind, docId, docTitle?, createdAt, protocolId }[]`.
  - API: `addJob(entry)`, `removeJob(jobId)`, `subscribe(listener)`, `getSnapshot()`.
  - Persistence: rehydrate from `localStorage` on load; write-through on add/remove; reconcile by dropping entries that 404 or are otherwise invalid.
  - Polling: always poll jobs whose backend `state` is `queued`/`running` (not gated on drawer open) so status stays current while multitasking.
- `app/src/components/AiJobsDrawer.tsx` (new; name may vary): global AI Jobs tracker UI (drawer/tray).
  - Persists across document switches (mounted at app shell, not inside `DocumentView`).
  - Shows list of tracked jobs with live state; selecting a job renders `JobResultPanel jobId=...` for output/details.
- `app/src/components/CommandPalette.tsx` (new): generic Command Palette modal.
  - Props: `open: boolean`, `title?: string`, `actions: CommandPaletteAction[]`, `onAction(actionId)`, `onClose()`.
  - Keyboard: `Enter` runs highlighted action, `ArrowUp/Down` navigates, `Escape` closes; filtering input filters by `label` + `keywords`.
  - Accessibility: `role="dialog"`, `aria-modal="true"`, focus input on open, restore focus on close.
- `CommandPaletteAction` (new type; colocated with component):
  - `{ id: string; label: string; description?: string; keywords?: string[]; disabled?: boolean }`
- `app/src/components/DocumentView.tsx` (update): "Summarize" becomes "AI Actions" (or keeps label) but opens the Command Palette.
  - Action allowlist (local const): only `doc_summarize` for this WP (no user-provided job_kind strings).
  - Hotkeys: support both `Ctrl/Cmd+K` (primary) and `Ctrl/Cmd+Shift+P` (fallback) to open palette while a document is selected.
  - On action run: call `createJob("doc_summarize", "doc-proto-001", documentId, jobInputs)` immediately (backend is the queue) and add returned `job_id` to the global tracker store.
  - `jobInputs` MUST include a valid `DocsAiJobProfile` per Master Spec 2.6.6.6.4 / invariant at Handshake_Master_Spec_v02.117.md:8831 (min: `{ doc_id: documentId, selection: null, layer_scope: "Document" }`).
  - If adding a user-tweakable instructions field: keep it explicit/allowlisted (single string), and include it in `job_inputs` as additive metadata (backend may ignore today; still captured for traceability).
- `app/src/lib/api.ts` (update): extend `createJob` to accept optional `job_inputs` while keeping the current API-layer pattern (no `fetch` in components).
  - When sending `job_inputs`, still send `doc_id` in the request body so the backend can attach `entity_refs` for workspace/document.
- `app/src/components/DocumentView.test.tsx` (update/add): verify palette opens (hotkey/button) and triggers `createJob` with current `documentId` and `job_inputs` containing the `DocsAiJobProfile` fields.
- Decisions (locked):
  - Polling: always poll queued/running jobs.
  - Persistence: store job tracker list in `localStorage` (rehydrate on load).
- Notes:
- Frontend-only: do not touch `src/backend/**`.
- Security/guardrail: palette uses a fixed allowlist of actions; no freeform `job_kind` strings from user input.
- UI state hygiene: global job tracker is not cleared on document switch; palette state should reset on document switch to avoid stale action context.

## IMPLEMENTATION
- Added a global AI jobs tracker store with `localStorage` persistence and polling for queued/running jobs (`app/src/state/aiJobs.ts`).
- Added a global AI Jobs drawer/tray UI mounted at the app shell and reusing `JobResultPanel` for output (`app/src/components/AiJobsDrawer.tsx`, `app/src/App.tsx`).
- Added a Command Palette modal for explicit actions and wired hotkeys + editor button to open it (`app/src/components/CommandPalette.tsx`, `app/src/components/DocumentView.tsx`).
- Implemented "Summarize document" action to call `createJob("doc_summarize", "doc-proto-001", documentId, jobInputs)` and track the created job id globally.
  - `jobInputs` includes the required `DocsAiJobProfile` minimum: `{ doc_id, selection: null, layer_scope: "Document" }`.
- Updated API helper to accept and send `job_inputs` while keeping component fetches centralized (`app/src/lib/api.ts`).
- Refactored `JobResultPanel` into a presentational component used by the global job tracker (`app/src/components/JobResultPanel.tsx`).
- Updated tests to cover palette-driven job creation and the required `job_inputs` fields (`app/src/components/DocumentView.test.tsx`).

## HYGIENE
- Commands run:
  - `just pre-work WP-1-AI-UX-Actions-v2`
  - `pnpm -C app run lint`
  - `pnpm -C app test`
  - `just validate` (fails in this repo due to `cargo deny` expecting a root `Cargo.toml`)
  - `just post-work WP-1-AI-UX-Actions-v2`
- Manual smoke:
  - Not run in this session (see TEST_PLAN for steps).

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `app/src/App.css`
- **Start**: 1
- **End**: 1310
- **Line Delta**: 288
- **Pre-SHA1**: `dfce683c0d45d8ede2f1068268b0bf717edbb864`
- **Post-SHA1**: `65a2f2b004901e6492ca703fa9df8acbc2f221cd`
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

- **Target File**: `app/src/App.tsx`
- **Start**: 1
- **End**: 200
- **Line Delta**: 2
- **Pre-SHA1**: `7d7a8db8bd7c4f89d0bde853d9da03c696e7e790`
- **Post-SHA1**: `4c23aa346b2c4fff3c5231c40c92e6bd3ca2a20f`
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

- **Target File**: `app/src/components/AiJobsDrawer.tsx`
- **Start**: 1
- **End**: 134
- **Line Delta**: 134
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `3e4ab5dbdd276a9378cf43197591bfcb7b8d0cdd`
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

- **Target File**: `app/src/components/CommandPalette.tsx`
- **Start**: 1
- **End**: 135
- **Line Delta**: 135
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `f66c586b543f14b8e129da5d798de4d2b094e7dc`
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

- **Target File**: `app/src/components/DocumentView.test.tsx`
- **Start**: 1
- **End**: 132
- **Line Delta**: 48
- **Pre-SHA1**: `9b7b6df4fe046f22a69d77cecc02a78deac28d92`
- **Post-SHA1**: `29521a691459b2baeda1225de955bf614fa33aea`
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

- **Target File**: `app/src/components/DocumentView.tsx`
- **Start**: 1
- **End**: 486
- **Line Delta**: 81
- **Pre-SHA1**: `654169a94269ddfc4e890a38c3fac0bf32badb82`
- **Post-SHA1**: `c83d4ebd0807390ccfabc85850a87d269b27325f`
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

- **Target File**: `app/src/components/JobResultPanel.tsx`
- **Start**: 1
- **End**: 61
- **Line Delta**: -30
- **Pre-SHA1**: `7fd35717362d078eb5b0dcbab4e743a03ab641ae`
- **Post-SHA1**: `85261e65442853eb152614d4f92b0f6ded496087`
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

- **Target File**: `app/src/lib/api.ts`
- **Start**: 1
- **End**: 642
- **Line Delta**: 9
- **Pre-SHA1**: `f2fc616e04d8a810339ffbe3640494cb4adc9eb9`
- **Post-SHA1**: `62268afad1bf2f2290028c010aeab1efd1dfb576`
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

- **Target File**: `app/src/state/aiJobs.ts`
- **Start**: 1
- **End**: 172
- **Line Delta**: 172
- **Pre-SHA1**: `da39a3ee5e6b4b0d3255bfef95601890afd80709`
- **Post-SHA1**: `3674f5a8216e3112280fe69ceb5fd2d3ecdc2af4`
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

- **Timestamp**: 2026-01-26T02:56:57.600Z
- **Operator**: CODER (GPT-5.2)
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.117.md
- **Notes**: Manifest values captured from staged (INDEX) content.

## STATUS_HANDOFF
- Current WP_STATUS: Implementation complete; staged and ready for commit after Operator authorization.
- What changed in this update:
  - Added Command Palette entrypoint (hotkeys + button) and implemented "Summarize document" action using `doc_summarize` with required `job_inputs` `DocsAiJobProfile`.
  - Added global AI Jobs tracker UI that persists across document switches and reloads; reuses `JobResultPanel` for output.
- Next step / handoff hint:
  - If approved, commit staged changes on `feat/WP-1-AI-UX-Actions-v2` and request Validator review/merge.

## EVIDENCE
- `just pre-work WP-1-AI-UX-Actions-v2`
  ```text
  Checking Phase Gate for WP-1-AI-UX-Actions-v2...
  ? GATE PASS: Workflow sequence verified.

  Pre-work validation for WP-1-AI-UX-Actions-v2...

  Check 1: Task packet file exists
  PASS: Found WP-1-AI-UX-Actions-v2.md

  Check 2: Task packet structure
  PASS: All required fields present

  Check 2.7: Technical Refinement gate
  PASS: Refinement file exists and is approved/signed

  Check 2.8: WP checkpoint commit gate

  Check 3: Deterministic manifest template
  PASS: Manifest fields present
  PASS: Gates checklist present

  ==================================================
  Pre-work validation PASSED

  You may proceed with implementation.
  ```

- `pnpm -C app run lint`
  ```text
  > app@0.1.0 lint D:\\Projects\\LLM projects\\wt-WP-1-AI-UX-Actions-v2\\app
  > eslint src --ext .ts,.tsx
  ```

- `pnpm -C app test`
  ```text
  NOTE: vitest output includes ANSI escape sequences + unicode checkmarks; omitted to keep this packet ASCII-only.
  Summary excerpt:
  - Test Files: 5 passed (5)
  - Tests: 11 passed (11)
  ```

- `just validate` (expected by TEST_PLAN for MEDIUM risk)
  ```text
  NOTE: fails in this repo because `cargo deny` is invoked at repo root, but there is no root Cargo.toml.
  Excerpt:
  cargo deny check advisories licenses bans sources
  2026-01-26 02:49:49 [ERROR] the directory D:\\Projects\\LLM projects\\wt-WP-1-AI-UX-Actions-v2 doesn't contain a Cargo.toml file
  error: Recipe `validate` failed on line 44 with exit code 1
  ```

- `just post-work WP-1-AI-UX-Actions-v2`
  ```text
  Checking Phase Gate for WP-1-AI-UX-Actions-v2...
  ? GATE PASS: Workflow sequence verified.

  Post-work validation for WP-1-AI-UX-Actions-v2 (deterministic manifest + gates)...

  Check 1: Validation manifest present

  Check 2: Manifest fields

  Check 3: File integrity (per manifest entry)

  Check 4: Git status

  ==================================================
  Post-work validation PASSED with warnings

  Warnings:
    1. Manifest[3]: Could not load HEAD version (new file or not tracked): app\\src\\components\\AiJobsDrawer.tsx
    2. Manifest[4]: Could not load HEAD version (new file or not tracked): app\\src\\components\\CommandPalette.tsx
    3. Manifest[9]: Could not load HEAD version (new file or not tracked): app\\src\\state\\aiJobs.ts

  You may proceed with commit.
  ? ROLE_MAILBOX_EXPORT_GATE PASS
  fatal: path 'app/src/components/AiJobsDrawer.tsx' exists on disk, but not in 'HEAD'
  fatal: path 'app/src/components/CommandPalette.tsx' exists on disk, but not in 'HEAD'
  fatal: path 'app/src/state/aiJobs.ts' exists on disk, but not in 'HEAD'
  ```

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
