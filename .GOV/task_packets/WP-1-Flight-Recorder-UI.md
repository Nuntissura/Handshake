# Task Packet: WP-1-Flight-Recorder-UI

## Metadata
- TASK_ID: WP-1-Flight-Recorder-UI
- DATE: 2025-12-19
- REQUESTOR: User
- AGENT_ID: Gemini-2.0-Flash
- ROLE: Orchestrator
- **Status:** Ready for Dev
- USER_SIGNATURE: ilja

---

## ðŸ•µï¸ CODE ARCHAEOLOGY & ALIGNMENT NOTE
**Reason:** Strategic Audit for Phase 1 closure.
**Authority [CX-598]:** The Roadmap is only a pointer. The **Master Spec Main Body** (Â§1-6, Â§9-11) is the sole definition of "Done."
**Procedure:** 
1. Validator/Coder must search for Flight Recorder components and DuckDB wiring.
2. Verify implementation matches **100% of the technical rules, schemas, and invariants** found in the Main Body (Â§11.5 Flight Recorder / Â§10.5 Operator Consoles).
3. Surface-level compliance with roadmap bullets (Â§7.6.3.5) is insufficient. Implementation must support typed events, traceability fields, and DuckDB persistence.
4. If 100% alignment exists -> **PASS**. Otherwise -> **FAIL**.

---

## Scope
- **What**: Implement a frontend view ("Job History" / "Flight Recorder") to visualize the AI events logged in DuckDB.
- **Why**: Provide visibility into the AI's actions, crucial for the "Glass Box" philosophy and debugging Phase 1 features.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/api/flight_recorder.rs (New)
  * src/backend/handshake_core/src/api/mod.rs
  * src/backend/handshake_core/src/flight_recorder.rs
  * app/src/lib/api.ts
  * app/src/components/FlightRecorderView.tsx (New)
  * app/src/App.tsx (Route/Navigation)
- **OUT_OF_SCOPE**:
  * Complex filtering/search of logs (basic chronological list is sufficient).
  * Real-time streaming of logs (polling is fine).

## Quality Gate
- **RISK_TIER**: MEDIUM
- **TEST_PLAN**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app run lint
  pnpm -C app test
  node .GOV/scripts/validation/post-work-check.mjs WP-1-Flight-Recorder-UI
  ```
- **DONE_MEANS**:
  * `GET /api/flight_recorder` endpoint returns recent events from DuckDB.
  * A new "Flight Recorder" or "History" tab/sidebar item exists in the UI.
  * The view lists AI events (job start, completion, etc.) with timestamps and payloads.
  * All tests pass.
- **ROLLBACK_HINT**:
  ```bash
  git checkout app/src/App.tsx src/backend/handshake_core/src/api/mod.rs
  rm src/backend/handshake_core/src/api/flight_recorder.rs app/src/components/FlightRecorderView.tsx
  ```

## BOOTSTRAP (Coder Work Plan)
- **FILES_TO_OPEN**:
  * Handshake_Master_Spec_v02.50.md (Â§7.6.3 Item 5)
  * src/backend/handshake_core/src/flight_recorder.rs
  * src/backend/handshake_core/src/main.rs
  * app/src/lib/api.ts
  * app/src/App.tsx
- **SEARCH_TERMS**:
  * "DuckDbConnection"
  * "log_event"
  * "flight_recorder"
  * "api::routes"
- **RUN_COMMANDS**:
  ```bash
  cargo check --manifest-path src/backend/handshake_core/Cargo.toml
  pnpm -C app test
  ```
- **RISK_MAP**:
  * "DuckDB concurrency lock" -> Backend Database
  * "Large payload rendering" -> Frontend Performance
  * "Date formatting mismatch" -> UI Display

## Authority
- **SPEC_CURRENT**: .GOV/roles_shared/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Task Board**: .GOV/roles_shared/TASK_BOARD.md
- **Logger**: Optional (milestones/hard bugs)

## Notes
- **Implementation Detail**: Ensure the DuckDB connection is safely shared or cloned for the query. Use `fetch_all` equivalent for DuckDB.

## Validation
- cargo check --manifest-path src/backend/handshake_core/Cargo.toml -> PASS
- pnpm -C app run lint -> PASS
- pnpm -C app test -> PASS
- node .GOV/scripts/validation/post-work-check.mjs WP-1-Flight-Recorder-UI -> PASS

## Status / Handoff
- WP_STATUS: Completed
- What changed: Added API route for `/api/flight_recorder` (alias `/api/events` retained), frontend view polls and renders events; navigation tab present.
- Next step / handoff hint: Optional enhancements: add filters or pagination if log volume grows.

---

## VALIDATION REPORT â€” WP-1-Flight-Recorder-UI
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Flight-Recorder-UI.md (status: In Progress)
- Spec: Handshake_Master_Spec_v02.84.md (Packet incorrectly references STALE v02.50)

Files Checked:
- app/src/components/FlightRecorderView.tsx
- src/backend/handshake_core/src/api/flight_recorder.rs

Findings:
- **Spec Regression**: Packet references v02.50. MUST align with Â§11.5 of v02.84 (Event Shapes & Retention).
- **Evidence Mapping [CX-627]**: MISSING. Coder has not mapped the UI implementation to specific spec requirements.
- **Event Shape Compliance**: Implementation must be audited to ensure it correctly renders the `payload JSON` including the new traceability fields (`trace_id`, `actor_id`) required by the Red Hat Auditor protocol.
- **Hygiene**: `post-work-check.mjs` (L59) is a legacy check and does not substitute for manual evidence-based validation.

Risks & Suggested Actions:
- **RE-OPEN**. Ensure the UI supports the full diagnostic schema defined in Â§11.4/11.5.
- Add `EVIDENCE_MAPPING` to the task packet.

---

**Last Updated:** 2025-12-25
**User Signature Locked:** <pending>


## VALIDATION REPORT — WP-1-Flight-Recorder-UI
Verdict: FAIL

Scope Inputs:
- Task Packet: .GOV/task_packets/WP-1-Flight-Recorder-UI.md (status: Ready for Dev)
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






