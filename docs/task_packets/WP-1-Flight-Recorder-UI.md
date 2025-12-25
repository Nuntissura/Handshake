# Task Packet: WP-1-Flight-Recorder-UI

## Metadata
- TASK_ID: WP-1-Flight-Recorder-UI
- DATE: 2025-12-19
- REQUESTOR: User
- AGENT_ID: Gemini-2.0-Flash
- ROLE: Orchestrator
- **Status:** In Progress
- USER_SIGNATURE: <pending>

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
  node scripts/validation/post-work-check.mjs WP-1-Flight-Recorder-UI
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
  * Handshake_Master_Spec_v02.50.md (§7.6.3 Item 5)
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
- **SPEC_CURRENT**: docs/SPEC_CURRENT.md
- **Codex**: Handshake Codex v0.8.md
- **Task Board**: docs/TASK_BOARD.md
- **Logger**: Optional (milestones/hard bugs)

## Notes
- **Implementation Detail**: Ensure the DuckDB connection is safely shared or cloned for the query. Use `fetch_all` equivalent for DuckDB.

## Validation
- cargo check --manifest-path src/backend/handshake_core/Cargo.toml -> PASS
- pnpm -C app run lint -> PASS
- pnpm -C app test -> PASS
- node scripts/validation/post-work-check.mjs WP-1-Flight-Recorder-UI -> PASS

## Status / Handoff
- WP_STATUS: Completed
- What changed: Added API route for `/api/flight_recorder` (alias `/api/events` retained), frontend view polls and renders events; navigation tab present.
- Next step / handoff hint: Optional enhancements: add filters or pagination if log volume grows.
