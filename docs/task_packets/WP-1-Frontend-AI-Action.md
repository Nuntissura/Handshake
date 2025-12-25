# Task Packet: WP-1-Frontend-AI-Action

## Metadata
- TASK_ID: WP-1-Frontend-AI-Action
- DATE: 2025-12-18T21:17:35.492Z
- REQUESTOR: Orchestrator
- AGENT_ID: Gemini (Orchestrator)
- ROLE: Orchestrator
- **Status:** In Progress
- USER_SIGNATURE: <pending>

## Scope
- **What**: Implement a UI element (e.g., a button) that, when clicked, triggers a network request to the backend's POST /api/jobs endpoint.
- **Why**: To provide a frontend mechanism for users to invoke AI-driven tasks, adhering to the project's AI-native design principles.
- **IN_SCOPE_PATHS**:
  * `app/src/components/DocumentView.tsx`
  * `app/src/components/TiptapEditor.tsx`
  * `app/src/lib/api.ts`
- **OUT_OF_SCOPE**:
  * Any changes to the backend.
  * Complex UI for displaying job status or results.

## Quality Gate
- **RISK_TIER**: MEDIUM
- **TEST_PLAN**:
  ```bash
  pnpm -C app run lint
  pnpm -C app tauri dev
  # Open a document and click the new UI button.
  # Verification: 
  # - Check backend console output for new job.
  # - Check browser console for successful API call (200 OK).
  ```
- **DONE_MEANS**:
  * A button or similar UI element exists, and clicking it successfully sends a request and creates a job on the backend, confirmed via logs.
  * All lint checks pass.
  * A final report is provided using the specified format.
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-sha>
  # OR: Revert changes to the modified React components in `app/src/components/` and `app/src/lib/api.ts`.
  ```

## Bootstrap (Coder Work Plan) [BOOTSTRAP]
- **FILES_TO_OPEN**:
  * `app/src/components/DocumentView.tsx` (main view for a document)
  * `app/src/components/TiptapEditor.tsx` (the text editor component)
  * `app/src/lib/api.ts` (the existing library for backend calls)
  * `src/backend/handshake_core/src/api/jobs.rs` (for API contract)
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * docs/ARCHITECTURE.md
- **SEARCH_TERMS**:
  * `Tiptap`
  * `useEditor`
  * `invoke`
  * `fetch`
  * `request`
  * `createJob`
  * `/api/jobs`
- **RUN_COMMANDS**:
  ```bash
  pnpm -C app tauri dev
  pnpm -C app run lint
  ```
- **RISK_MAP**:
  * "API call fails -> Check for CORS, network errors, or malformed request body."
  * "Difficult to add a button to the editor -> `TiptapEditor` may have a rigid structure requiring careful integration."
  * "UI component difficult to modify -> The `TiptapEditor` component might have a complex structure."
  * "State management for button clicks is unclear -> Need to find the right place to handle the click event and API call."

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Latest Logger**: Handshake_logger_20251218_v3.3_20251218T204200.md
- **ADRs**: None

## Notes
- **Assumptions**: The current frontend build errors (HSK-P1-002-DEBUG.1) will be resolved before this task is truly actionable for the Coder.
- **Open Questions**: None
- **Dependencies**: Resolution of HSK-P1-002-DEBUG.1 (Frontend TypeScript Build Failures).
- **The final report is a critical part of this task.** The Orchestrator will use it to validate your work and to plan future refactoring and cleanup tasks based on your hygiene assessment. Please be thorough but concise in your observations.
- **QUALITY_RUBRIC**: Your work will be evaluated against the standard Coder Performance & Quality Rubric (defined in Handshake Codex v0.8.md).
