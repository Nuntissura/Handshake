# Task Packet: WP-1-AI-UX-Summarize-Display

## Metadata
- TASK_ID: WP-1-AI-UX-Summarize-Display
- ROLE: Orchestrator
- STATUS: Ready for Dev
- USER_SIGNATURE: ilja

---

## 🕵️ CODE ARCHAEOLOGY & ALIGNMENT NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (§1-6, §9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator/Coder must check summarization display logic.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (§7.6.3.7).
3. Surface-level compliance with roadmap bullets is insufficient. Every line of text in the Main Body section must be implemented.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## Executive Summary

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

---

## VALIDATION REPORT — WP-1-AI-UX-Summarize-Display
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-AI-UX-Summarize-Display.md (status: In Progress)
- Spec: Handshake_Master_Spec_v02.84.md (Packet incorrectly references STALE v02.50)

Files Checked:
- app/src/components/JobResultPanel.tsx
- src/backend/handshake_core/src/api/jobs.rs

Findings:
- **Spec Regression**: References STALE v02.50. MUST align with v02.84.
- **Evidence Mapping [CX-627]**: MISSING.
- **RDD Model Compliance**: Implementation must prove that summaries are treated as **DerivedContent** (Section 2.2.2.2) and rendered at the **Display** layer, not written back into Raw without explicit intent.
- **Traceability**: The UI should display the `job_id` and `model_id` for the result to meet observability requirements.

Risks & Suggested Actions:
- **RE-OPEN**. Update packet to reference v02.84.
- Add `EVIDENCE_MAPPING`.

---

**Last Updated:** 2025-12-25
**User Signature Locked:** <pending>


## VALIDATION REPORT � WP-1-AI-UX-Summarize-Display
Verdict: FAIL

Scope Inputs:
- Task Packet: docs/task_packets/WP-1-AI-UX-Summarize-Display.md (status: Ready for Dev)
- Spec: (not provided)

Findings:
- Packet incomplete [CX-573]: missing required fields (RISK_TIER, TEST_PLAN, DONE_MEANS, BOOTSTRAP, AUTHORITY); USER_SIGNATURE pending. Pre-flight gate blocks validation.
- No implementation evidence provided; validation halted until packet completeness and evidence mapping exist.

Hygiene / Forbidden Patterns:
- Not run (blocked by pre-flight failure).

Tests:
- Not run (TEST_PLAN missing).

Reason for FAIL:
- Workflow pre-flight failed; WP returned to Ready for Dev pending packet completion and implementation evidence.



