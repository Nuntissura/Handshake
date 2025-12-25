# Task Packet: WP-1-AI-Integration-Baseline

## Metadata
- TASK_ID: WP-1-AI-Integration-Baseline
- DATE: 2025-12-19
- REQUESTOR: User
- AGENT_ID: Gemini-2.0-Flash
- ROLE: Orchestrator
- **Status:** In Progress
- USER_SIGNATURE: <pending>

## Scope
- **What**: Implement the core LLM client abstraction and integrate it into the AI Job/Workflow system for basic document actions (Summary/Chat).
- **Why**: Move from mock AI jobs to real LLM interactions as required by Phase 1 MVP.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/llm.rs
  * src/backend/handshake_core/src/api/jobs.rs
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/main.rs
  * src/backend/handshake_core/Cargo.toml
- **OUT_OF_SCOPE**:
  * Full Tiptap rich-text persistence (stay with plain text for now).
  * Complex multi-agent orchestration.
  * Image generation (Phase 1 extension).

## Quality Gate
- **RISK_TIER**: HIGH
- **TEST_PLAN**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app test
  node scripts/validation/post-work-check.mjs WP-1-AI-Integration-Baseline
  ```
- **DONE_MEANS**:
  * `LLMClient` trait exists in `src/backend/handshake_core/src/llm.rs`.
  * `OllamaClient` implementation exists.
  * `/api/jobs` accepts a document ID and performs a real LLM call (summarization).
  * Workflow engine logs real LLM events to Flight Recorder.
  * All tests pass.
- **ROLLBACK_HINT**:
  ```bash
  git revert HEAD
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.50.md
  * src/backend/handshake_core/src/api/jobs.rs
  * src/backend/handshake_core/src/workflows.rs
  * src/backend/handshake_core/src/main.rs
- **SEARCH_TERMS**:
  * "LLMClient"
  * "/api/jobs"
  * "start_workflow_for_job"
  * "Ollama"
- **RUN_COMMANDS**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "LLM runtime not available" -> Model Runtime Layer
  * "Document context too large" -> AI Job Model
  * "Network failure to Ollama" -> LLM Client

## Authority
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Latest Logger**: Handshake_logger_20251218.md

## Notes
- **Assumptions**: We assume Ollama is running locally on the default port (11434) for real testing, but we will implement a mock client for CI/automated tests.
