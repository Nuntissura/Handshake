# Handshake Phase 0.5 Closure — Diagnostic Baseline v0.1

## 1. Purpose of Phase 0.5

- Establish a runnable desktop vertical slice (Tauri + React) aligned with Phase 0 foundations.
- Lock in document/canvas editors with basic persistence and save/load pathways.
- Add minimal but usable diagnostics (Debug/Status panel, WorkspaceSidebar error UX).
- Provide smoke-level tests to catch obvious regressions before Phase 1.

## 2. What is implemented now (Baseline)

### 2.1 Application surfaces

- WorkspaceSidebar:
  - Lists workspaces, documents, and canvases.
  - On fetch failure, shows a “refresh failed but data is safe” banner and a Retry action; does not clear existing items.
- DocumentView:
  - Tiptap-based editor for plain-text blocks.
  - Save pipeline: getDocument → blocksToTiptap → Tiptap → tiptapToBlocks → updateDocumentBlocks.
- CanvasView:
  - Excalidraw-based canvas with shapes/text/freedraw/arrows/images.
  - Save/load pipeline via canvasToElements / elementsToGraph and node/edge/file snapshots.
- DebugPanel:
  - Shows backend health + DB status and recent debug events (doc/canvas), with refresh controls and log tail.

### 2.2 Tests and diagnostics

- App.test.tsx — app shell renders.
- DebugPanel.test.tsx — healthy + error health states, DB status, recent events.
- DocumentView.test.tsx — edit text and save updates document blocks (note: contentEditable/act warnings known).
- CanvasSerialization.test.ts — canvasToElements/elementsToGraph round-trip preserves IDs and key props.
- WorkspaceSidebar.test.tsx — success path (no error banner) and failure path (error banner + Retry).
- Warnings: contentEditable + act(...) warnings in DocumentView test are known/accepted for this baseline.

### 2.3 Invariants locked by this baseline

- Doc/canvas persistence shape is stable (canvas serialization helpers, document block save pipeline).
- WorkspaceSidebar must not clear workspaces on fetch start or error; must retain last-known list and offer Retry on failure.
- DebugPanel must continue to expose health status, DB status, and recent debug events.
- All listed tests remain green for any Phase 1 work.

## 3. Known gaps and risks

- Test coverage is thin and largely happy-path; no E2E/UI automation yet.
- DocumentView tests rely on mocked Tiptap and emit contentEditable/act warnings.
- No AI job orchestration or Phase 1 features wired yet.
- No automated coverage of multi-workspace flows or richer editor/canvas behaviours beyond current snapshots.

## 4. Proposed Phase 1 vertical slice — Read-only AI Doc Summary Panel

- Add a read-only “AI summary” panel adjacent to DocumentView that summarizes the currently selected document.
- Backend runs a doc-summary AI job (no direct frontend model calls); job is read-only on document content.
- AI calls routed through the existing LLM client abstraction (no provider-specific code in the job).
- Rough API shape: GET `/documents/{id}/ai-summary` or POST `/ai/jobs/doc-summary` with `{ document_id }`, returning `{ summary, key_points }`.
- Constraints: no document writes by AI in Phase 1; integrate with observability (log AI job launches/results into DebugPanel/debug events); must not break existing Phase 0.5 tests—new tests should extend coverage.

## 5. Suggested Phase 1 work packets

- WP-1.0-DocSummary-Backend — implement doc-summary AI job + REST endpoint via LLM client abstraction.
- WP-1.0-DocSummary-Frontend — add summary panel next to DocumentView and wire it to the new endpoint.
- WP-1.0-DocSummary-Tests — backend unit tests + frontend tests for summary rendering and error states.
- WP-1.0-Observability — log AI job launches/results into DebugPanel/recent events with health/error surfacing.
- WP-1.0-Validation — tighten regression coverage around existing Phase 0.5 surfaces (doc/canvas round-trips, sidebar error UX) plus new summary flows.
