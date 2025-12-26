# Task Packet: WP-1-Frontend-Build-Debug

## Metadata
- TASK_ID: WP-1-Frontend-Build-Debug
  - DATE: 2025-12-18T21:20:12.436Z
  - REQUESTOR: Orchestrator
  - AGENT_ID: Gemini (Orchestrator)
  - ROLE: Orchestrator
  - **Status:** Ready for Dev
  - USER_SIGNATURE: ilja
  ## Scope
- **What**: Resolve all TypeScript build failures in the frontend application.
- **Why**: The frontend build is currently blocked, preventing further development on the UI. This task is critical to unblock all frontend feature implementation.
- **IN_SCOPE_PATHS**:
  * All `.ts` and `.tsx` files under `app/src/`.
  * `app/tsconfig.json`
  * `app/vite.config.ts`
  * `app/package.json`
- **OUT_OF_SCOPE**:
  * Any changes to the backend (`src/backend/handshake_core/`).
  * Do not implement the original AI button feature; focus only on fixing the build.

## Quality Gate
- **RISK_TIER**: HIGH
- **TEST_PLAN**:
  ```bash
  pnpm -C app build
  pnpm -C app test
  ```
- **DONE_MEANS**: The `pnpm -C app build` command completes with an exit code of 0 and no TypeScript errors reported in the console.
- **ROLLBACK_HINT**:
  ```bash
  git revert <commit-sha>
  # OR: Revert all changes made to the app/ directory during this task.
  ```

## Bootstrap (Coder Work Plan) [BOOTSTRAP]
- **FILES_TO_OPEN**:
  * `app/src/App.test.tsx`
  * `src/App.test.tsx`
  * `src/components/CanvasSerialization.test.ts`
  * `src/components/CanvasView.tsx`
  * `src/components/DebugPanel.test.tsx`
  * `src/components/DocumentView.test.tsx`
  * `src/components/DocumentView.tsx`
  * `src/components/WorkspaceSidebar.test.tsx`
  * `app/tsconfig.json`
  * `app/tsconfig.node.json`
  * `app/vite.config.ts`
  * `app/src/setupTests.ts`
  * `app/src/state/debugEvents.ts`
  * `app/package.json`
- **SEARCH_TERMS**:
  * `it`
  * `expect`
  * `describe`
  * `DebugEventType`
  * `FileId`
  * `DataURL`
  * `ExcalidrawLinearElement`
  * `pressures`
- **RUN_COMMANDS**:
  ```bash
  pnpm -C app build
  ```
- **RISK_MAP**:
  * "Fixing one type error reveals another -> Follow the compiler's feedback iteratively."

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Latest Logger**: Handshake_logger_20251218_v3.3_20251218T204200.md
- **ADRs**: None

## Notes
- **Assumptions**: None
- **Open Questions**: None
- **Dependencies**: None
- **The build is failing due to two primary categories of TypeScript errors:**
    1.  **Missing Test Types:** The compiler cannot find the definitions for test-related globals like `it`, `describe`, and `expect`. The solution likely involves correcting the `tsconfig.json` or `vite.config.ts` files to properly include the `vitest` type definitions.
    2.  **Strict Type Mismatches:** There are numerous errors where a general `string` is being assigned to a more specific or "branded" type (like `FileId`), or where an object property does not exist on a given type. These must be fixed by either updating the type definitions (e.g., adding `"ai-job"` to the `DebugEventType` union) or by ensuring the code provides the correctly typed data.
- Your mission is to resolve these build-blocking errors. Do not proceed with the original feature implementation.
- **REPORTING**: A mandatory report in the `[[codex]]...[[/codex]]` format is required. It must detail:
    1.  **Changes**: A summary of all fixes for the TypeScript errors.
    2.  **Validation**: Confirmation that `pnpm -C app build` passes, including any relevant console output.
    3.  **Hygiene & Structure Assessment**: Any observations about the cause of the type errors (e.g., missing type definitions, overly strict configs, etc.).
    4.  **QUALITY_RUBRIC**: Your work will be evaluated against the standard Coder Performance & Quality Rubric (defined in Handshake Codex v0.8.md).

---

## VALIDATION REPORT — WP-1-Frontend-Build-Debug
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-Frontend-Build-Debug.md (status: In Progress)
- Spec: CI/Build Hygiene (§11.7.4)

Files Checked:
- app/tsconfig.json
- app/package.json

Findings:
- **Evidence Mapping [CX-627]**: MISSING. Although the board history suggests this was fixed, the packet contains NO report detailing the specific TypeScript resolutions or evidence that `pnpm build` passed.
- **Protocol Violation**: The mandatory `[[codex]]...[[/codex]]` report requested in the NOTES section is missing.

Risks & Suggested Actions:
- **RE-OPEN**. Coder must supply the implementation report and evidence mapping. A passing `just validate` run is required to close this.

---

**Last Updated:** 2025-12-25
**User Signature Locked:** <pending>



