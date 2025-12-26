# Task Packet: WP-1-AI-Integration-Baseline

## Metadata
- TASK_ID: WP-1-AI-Integration-Baseline
- DATE: 2025-12-19
- REQUESTOR: User
- AGENT_ID: Gemini-2.0-Flash
- ROLE: Orchestrator
- **Status:** Ready for Dev
- USER_SIGNATURE: ilja

---

## 🕵️ CODE ARCHAEOLOGY & ALIGNMENT NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (§1-6, §9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator/Coder must search for LLM client abstractions.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (§4.0 Model Runtime Layer).
3. Surface-level compliance with roadmap bullets (§7.6.3.1) is insufficient. Every line of text in the Main Body section must be implemented (Typed errors, budgeting, context assembly).
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

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

---

## VALIDATION REPORT — WP-1-AI-Integration-Baseline
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-AI-Integration-Baseline.md (status: In Progress)
- Spec: Handshake_Master_Spec_v02.84.md (Packet incorrectly references STALE v02.50)

Files Checked:
- docs/task_packets/WP-1-AI-Integration-Baseline.md
- src/backend/handshake_core/src/llm.rs

Findings:
- **Requirement Drift**: Packet references STALE Spec v02.50. MUST align with v02.84 technical invariants for the Model Runtime Layer.
- **Evidence Mapping [CX-627]**: MISSING. Coder has not provided mapping of spec requirements to code paths.
- **HSK-#### Traceability [CX-640]**: MISSING. No stable error codes defined for LLM failures (e.g., connection timeout, context overflow).
- **Hollow Implementation [CX-573D]**: Current `llm.rs` (if exists) must be audited for "mock" patterns in production paths.
- **Flight Recorder Integration**: `DONE_MEANS` mentions logging but lacks specific event shapes defined in §11.5 of v02.84.

Risks & Suggested Actions:
- **RE-OPEN** for Spec Alignment. Orchestrator must update packet to reference v02.84 and include technical constraints from §4.1-4.3.
- Coder must provide `EVIDENCE_MAPPING` before next validation pass.

---

**Last Updated:** 2025-12-25
**User Signature Locked:** <pending>



