# Task Packet: WP-1-AI-UX-Summarize-Display

## Metadata
- TASK_ID: WP-1-AI-UX-Summarize-Display
- DATE: 2025-12-19
- REQUESTOR: User
- AGENT_ID: Gemini-2.0-Flash
- ROLE: Orchestrator
- STATUS: Completed

## Scope
- **What**: Implement a "Job Result" retrieval API and a UI component to display generated document summaries.
- **Why**: Complete the end-to-end user loop for AI document actions in Phase 1 MVP.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/api/jobs.rs
  * app/src/lib/api.ts
  * app/src/components/DocumentView.tsx
  * app/src/components/JobResultPanel.tsx (New component)
- **OUT_OF_SCOPE**:
  * Persistent storage of the summary back into document blocks (keep as transient job output for now).
  * Multi-job history view (focus on the *current* job result first).

## Quality Gate
- **RISK_TIER**: MEDIUM
- **TEST_PLAN**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app run lint
  pnpm -C app test
  node scripts/validation/post-work-check.mjs WP-1-AI-UX-Summarize-Display
  ```
- **DONE_MEANS**:
  * `GET /api/jobs/:id` endpoint returns `AiJob` with `job_outputs`.
  * `DocumentView` displays a "Summary" section containing the LLM response after a job finishes.
  * The UI correctly handles "pending", "completed", and "failed" job states with appropriate messaging.
  * All tests pass.
- **ROLLBACK_HINT**:
  ```bash
  git checkout app/src/components/DocumentView.tsx src/backend/handshake_core/src/api/jobs.rs
  rm app/src/components/JobResultPanel.tsx
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * Handshake_Master_Spec_v02.50.md (§7.6.3 Item 5 & 7)
  * src/backend/handshake_core/src/api/jobs.rs
  * src/backend/handshake_core/src/models.rs
  * app/src/lib/api.ts
  * app/src/components/DocumentView.tsx
- **SEARCH_TERMS**:
  * "AiJob"
  * "create_new_job"
  * "doc_summarize"
  * "job_outputs"
- **RUN_COMMANDS**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app test
  ```
- **RISK_MAP**:
  * "Polling creates high load" -> Frontend Logic
  * "JSON parse error on job_outputs" -> API Layer
  * "Summary not cleared on doc switch" -> UI State

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Latest Logger**: Handshake_logger_20251218.md

## Notes
- **Implementation Detail**: Use a simple polling strategy (e.g., `setInterval` or recursive `setTimeout`) in the frontend to check job status if the initial creation returns a "running" or "queued" status.