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
  - app/src/components/DocumentView.tsx
  - app/src/components/TiptapEditor.tsx
  - app/src/components/JobResultPanel.tsx
  - app/src/components/AiJobsDrawer.tsx (new; name may vary)
  - app/src/lib/api.ts
  - app/src/state/aiJobs.ts (new; name may vary)
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
