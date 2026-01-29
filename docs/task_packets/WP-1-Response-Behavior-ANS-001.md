# Task Packet: WP-1-Response-Behavior-ANS-001

## METADATA
- TASK_ID: WP-1-Response-Behavior-ANS-001
- WP_ID: WP-1-Response-Behavior-ANS-001
- BASE_WP_ID: WP-1-Response-Behavior-ANS-001 (stable ID without `-vN`; equals WP_ID for non-revision packets; if WP_ID includes `-vN`, override to the base ID)
- DATE: 2026-01-28T19:59:32.766Z
- REQUESTOR: ilja
- AGENT_ID: orchestrator2
- ROLE: Orchestrator
- CODER_MODEL: GPT-5.2 (Codex CLI)
- CODER_REASONING_STRENGTH: HIGH
- **Status:** In Progress
- RISK_TIER: MEDIUM
- USER_SIGNATURE: ilja280120261944

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: docs/refinements/WP-1-Response-Behavior-ANS-001.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SCOPE
- What: Implement ANS-001 (Response Behavior Contract) capture + persistence for the `frontend` runtime model role, plus basic UI inspection (hide/show inline + side-panel timeline viewer), plus leak-safe runtime chat telemetry events (`FR-EVT-RUNTIME-CHAT-101..103`) per `Handshake_Master_Spec_v02.121.md` \u00A711.5.10 / `EXEC-060`.
- Why: Provide deterministic operator inspection/debugging of the frontend model without relying on chat-as-state or consuming tokens to restate chat history, and emit leak-safe runtime telemetry for append + ANS-001 validation.
- IN_SCOPE_PATHS:
  - app/src-tauri/Cargo.toml
  - app/src-tauri/src/lib.rs
  - app/src-tauri/src/session_chat_log.rs
  - app/src/App.css
  - app/src/App.tsx
  - app/src/components/DebugPanel.tsx
  - app/src/components/operator/Ans001TimelineDrawer.tsx
  - app/src/components/operator/index.ts
  - app/src/lib/api.ts
  - app/src/lib/ans001.ts
  - app/src/lib/crypto.ts
  - app/src/lib/sessionChat.ts
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/api/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/tests/runtime_chat_events_tests.rs (new)
- SCOPE_AMENDMENT (in-place edit):
  - 2026-01-29: extended scope to include backend Flight Recorder runtime chat events (approval: `OVERRIDE: allow in-place edit (CX-573B, CX-585A/C)` + signature `ilja290120260236`).
- OUT_OF_SCOPE:
  - src/backend/handshake_core/src/api/canvases.rs (owned by concurrent WP-1-Global-Silent-Edit-Guard)
  - src/backend/handshake_core/src/api/workspaces.rs (owned by concurrent WP-1-Global-Silent-Edit-Guard)
  - docs/task_packets/WP-1-Global-Silent-Edit-Guard.md (owned by concurrent WP-1-Global-Silent-Edit-Guard)
  - Full "work surface" UI vision (Kanban, project-wide trackers, model/agent overview, microtask viewer) beyond the basic ANS-001 panel/toggles
  - Cloud sync / upload of session chat logs
  - Applying this log feature to backend model roles (`orchestrator`, `worker`, `validator`)

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-Response-Behavior-ANS-001

pnpm -C app test
pnpm -C app lint
pnpm -C app build
# Optional (if supported in environment):
pnpm -C app tauri build

just cargo-clean
just test
just lint

just post-work WP-1-Response-Behavior-ANS-001
```

### DONE_MEANS
- For interactive chat sessions with runtime model role `frontend`, the runtime appends one JSON object per line to `{APP_DATA}/sessions/<session_id>/chat.jsonl` (append-only, crash-safe, no in-place edits) per spec `Handshake_Master_Spec_v02.121.md` \u00A72.10.4.
- Entries include `schema_version: "hsk.session_chat_log@0.1"` and are ordered by `(turn_index, created_at_utc, message_id)` ascending.
- For assistant messages with `model_role="frontend"`, the log entry includes `ans001` payload (or `null` only when response emission was blocked) and optional `ans001_validation` (leak-safe; no full content duplication).
- Flight Recorder (append): On every user/assistant message appended to the runtime session chat log, the runtime emits `FR-EVT-RUNTIME-CHAT-101` (leak-safe; no inline body) per spec \u00A711.5.10.
- Flight Recorder (EXEC-060): On every ANS-001 validation attempt for a `frontend` assistant message, the runtime emits `FR-EVT-RUNTIME-CHAT-102` containing `session_id`, `message_id`, `role`, `model_role`, `ans001_compliant`, `violation_clauses[]`, `body_sha256`, `ans001_sha256` (no inline content) per spec \u00A72.8.v02.13 (EXEC-060) and \u00A711.5.10.
- Flight Recorder (session close): When the interactive chat session is closed, the runtime emits `FR-EVT-RUNTIME-CHAT-103` per spec \u00A711.5.10.
- Flight Recorder (privacy): Events MUST NOT embed the `{APP_DATA}/sessions/<session_id>/chat.jsonl` filesystem path; store only `session_id` (spec \u00A711.5.10).
- UI: ANS-001 payload is hidden inline by default, with per-message expand/collapse and a global show-inline toggle (default OFF) per spec \u00A72.7.1.7.
- UI: a side-panel "ANS-001 Timeline" lists messages newest->oldest and can open the full ANS-001 payload for any assistant message (spec \u00A72.7.1.7 SHOULD).
- Raw log is not censored/softened/rewritten; any redaction is export-only and MUST NOT write back into the session chat log (spec \u00A72.10.4).
- Concurrency: this WP does not touch the files owned by WP-1-Global-Silent-Edit-Guard (listed in OUT_OF_SCOPE above).

### ROLLBACK_HINT
```bash
# N/A until implementation commits exist (coder adds concrete revert command at handoff).
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.121.md (recorded_at: 2026-01-28T19:59:32.766Z)
- SPEC_TARGET: docs/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.121.md 2.7.1.7 Presentation + Persistence (Frontend UI) (Normative) [ADD v02.121]
  - Handshake_Master_Spec_v02.121.md 2.8.v02.13 ANS-001 Invocation (EXEC-057 to EXEC-060)
  - Handshake_Master_Spec_v02.121.md 2.10.4 Frontend Session Chat Log (ANS-001) (Normative) [ADD v02.121]
  - Handshake_Master_Spec_v02.121.md 11.5.10 FR-EVT-RUNTIME-CHAT-101..103 (Frontend Conversation + ANS-001 Telemetry) (Normative) [ADD v02.121]
- Codex: Handshake Codex v1.4.md
- Task Board: docs/TASK_BOARD.md
- WP Traceability: docs/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - docs/START_HERE.md
  - docs/SPEC_CURRENT.md
  - docs/ARCHITECTURE.md
  - docs/refinements/WP-1-Response-Behavior-ANS-001.md
  - docs/task_packets/WP-1-Response-Behavior-ANS-001.md
  - Handshake_Master_Spec_v02.121.md
  - app/src/App.tsx
  - app/src/components/DebugPanel.tsx
  - app/src/components/operator/TimelineView.tsx
  - app/src-tauri/src/main.rs
  - app/src-tauri/src/lib.rs
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/api/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
- SEARCH_TERMS:
  - "ANS-001"
  - "session"
  - "appDataDir"
  - "Timeline"
  - "Flight Recorder"
  - "EXEC-060"
  - "FR-EVT-RUNTIME-CHAT"
  - "runtime_chat_message_appended"
  - "runtime_chat_ans001_validation"
  - "runtime_chat_session_closed"
  - "hsk.fr.runtime_chat@0.1"
- RUN_COMMANDS:
  ```bash
  pnpm -C app test
  pnpm -C app lint
  just test
  just lint
  ```
- RISK_MAP:
  - "raw session chat log contains secrets" -> "ensure local-only storage under {APP_DATA}; no auto-upload"
  - "log growth / retention" -> "disk usage; rotation/retention policy deferred (out of scope for this WP)"
  - "Flight Recorder telemetry leaks content" -> "leak-safe only: hashes/pointers; never inline bodies; never embed filesystem paths"
  - "Flight Recorder runtime chat event schema mismatch" -> "treat as hard failure; validate payload shape and required keys; add test coverage"

## SKELETON
- Proposed interfaces/types/contracts:
- Tauri (Rust) persistence + commands (app/src-tauri/**):
  - New module: app/src-tauri/src/session_chat_log.rs
  - State (managed via app.manage(...)):
    - SessionChatLogState { session_id: String (UUID), next_turn_index: Mutex<u64>, app_data_root: PathBuf }
  - Spec-aligned types (serde):
    - SessionChatRole = "user" | "assistant"
    - SessionChatLogEntryV0_1 (exact fields per spec 2.10.4):
      - schema_version: "hsk.session_chat_log@0.1"
      - session_id: string (uuid)
      - turn_index: number (1-based, strictly increasing)
      - created_at_utc: string (RFC3339 UTC)
      - message_id: string (uuid)
      - role: SessionChatRole
      - model_role?: "frontend" | null (present only for role="assistant")
      - content: string
      - ans001?: ResponseBehaviorContract | null (present only for role="assistant" AND model_role="frontend")
      - ans001_validation?: { compliant: boolean; violation_clauses: string[] } | null (optional)
    - SessionChatLogEntryV0_1Input (for command payload):
      - role, content (required)
      - model_role, ans001, ans001_validation (optional)
      - message_id (optional; if omitted server generates; server always fills turn_index + created_at_utc + session_id)
  - Commands (wired in app/src-tauri/src/lib.rs invoke_handler):
    - session_chat_get_session_id() -> string
    - session_chat_append(entry: SessionChatLogEntryV0_1Input) -> ()
    - session_chat_read(session_id: string, limit?: number) -> SessionChatLogEntryV0_1[]
  - Storage path + write semantics:
    - Resolve {APP_DATA} via app.path().app_data_dir()
    - File: {APP_DATA}/sessions/<session_id>/chat.jsonl
    - Ensure dir exists via create_dir_all
    - Append-only JSONL: write exactly 1 line (serde_json) + "\\n" per append call
    - Crash-safety: write line in a single write_all; then sync_data (best-effort)
    - Ordering: enforce monotonically increasing turn_index in-process (Mutex counter)
  - Time + IDs:
    - created_at_utc via time::OffsetDateTime::now_utc().format(Rfc3339)
    - message_id via uuid (new dependency) when missing

- Backend (Rust) Flight Recorder extensions (src/backend/handshake_core/src/flight_recorder/**):
  - New `FlightRecorderEventType` variants (string values MUST match spec \u00A711.5.10):
    - RuntimeChatMessageAppended => "runtime_chat_message_appended" (FR-EVT-RUNTIME-CHAT-101)
    - RuntimeChatAns001Validation => "runtime_chat_ans001_validation" (FR-EVT-RUNTIME-CHAT-102)
    - RuntimeChatSessionClosed => "runtime_chat_session_closed" (FR-EVT-RUNTIME-CHAT-103)
  - New runtime chat payload structs (for clarity + typed parsing at the API boundary):
    - RuntimeChatEventType = "runtime_chat_message_appended" | "runtime_chat_ans001_validation" | "runtime_chat_session_closed"
    - RuntimeChatEventV0_1 (spec \u00A711.5.10 canonical JSON shape):
      - schema_version: "hsk.fr.runtime_chat@0.1"
      - event_id: "FR-EVT-RUNTIME-CHAT-101|102|103" (string constant per event type)
      - ts_utc: RFC3339 UTC
      - session_id: UUID
      - type: RuntimeChatEventType
      - optional correlation: job_id, work_packet_id, spec_id, wsid
      - message context (for *_message_* and *_ans001_validation): message_id, role ("user"|"assistant"), model_role ("frontend"|null)
      - leak-safe hashes only: body_sha256, ans001_sha256 (NO inline content)
      - validation fields (for *_ans001_validation): ans001_compliant:boolean, violation_clauses:string[]
  - Strict payload validation (inside flight_recorder/mod.rs; reject unknown keys; reject leak vectors):
    - Add validators:
      - validate_runtime_chat_message_appended_payload(Value) -> Result<()>
      - validate_runtime_chat_ans001_validation_payload(Value) -> Result<()>
      - validate_runtime_chat_session_closed_payload(Value) -> Result<()>
    - Required enforcement for FR-EVT-RUNTIME-CHAT-102 (EXEC-060):
      - schema_version fixed == "hsk.fr.runtime_chat@0.1"
      - event_id fixed == "FR-EVT-RUNTIME-CHAT-102"
      - type fixed == "runtime_chat_ans001_validation"
      - require session_id/message_id UUID strings
      - require role == "assistant" and model_role == "frontend"
      - require body_sha256 + ans001_sha256 are 64-char hex sha256 (no inline content)
      - require ans001_compliant boolean and violation_clauses[] array of strings
      - reject unknown keys (require_allowed_keys) and reject any inline fields like "content", "body", "ans001", "chat_log_path" / any {APP_DATA} path-like fields
  - DuckDB sink behavior (duckdb.rs):
    - No schema changes required: store new events into existing events table with event_type + payload JSON.
    - Ensure the new event_type values are recorded and round-trip via list_events.

- Backend API ingestion route (src/backend/handshake_core/src/api/flight_recorder.rs + registered in api/mod.rs merge):
  - Add POST endpoint (new): POST /flight_recorder/runtime_chat_event
  - Accept JSON payload RuntimeChatEventV0_1 (deny unknown fields at the API boundary) and record as FlightRecorderEvent with event_type mapped from payload.type.
  - The backend MUST NOT accept inline message bodies or session chat log filesystem paths; only leak-safe hashes and identifiers are persisted.

- Hashing + emission responsibility (no inline content stored):
  - body_sha256 / ans001_sha256 are computed in the frontend (TypeScript) from the raw content / ANS-001 JSON and sent to the backend ingestion endpoint as hashes only.
  - Backend validators hard-reject payloads that attempt to include inline bodies, ANS-001 payloads, or {APP_DATA} paths (spec \u00A711.5.10 rules).

- Tests (backend; required by updated scope):
  - New integration test: src/backend/handshake_core/tests/runtime_chat_events_tests.rs
  - Proves:
    - Accepts valid FR-EVT-RUNTIME-CHAT-102 payload (exact required keys, valid sha256 hex, no inline content)
    - Rejects unknown extra keys
    - Rejects payloads containing inline content keys ("content"/"ans001") or any file path key ("chat_log_path"/"app_data_path")
    - Rejects invalid sha256 strings

- Frontend (React) types + UI inspector (app/src/**):
  - New TS types (one file; imported by UI + dev harness):
    - ResponseBehaviorContract (spec 2.7.1) (top-level shape + key nested types used by UI)
    - SessionChatRole, SessionChatLogEntryV0_1 (exact per spec 2.10.4)
  - New Tauri invoke wrappers (like app/src/lib/fonts.ts):
    - sessionChatGetSessionId(): Promise<string>
    - sessionChatAppend(entry: SessionChatLogEntryV0_1Input): Promise<void>
    - sessionChatRead(sessionId: string, limit?: number): Promise<SessionChatLogEntryV0_1[]>
  - UI: ANS-001 Timeline side drawer/panel (available across views):
    - Toggle open via header button "ANS-001 Timeline"
    - List messages newest->oldest (reverse of read order)
    - Selecting a row shows message content + ANS-001 payload (assistant + model_role="frontend" only)
    - Default-hide ANS-001 inline:
      - global toggle Show ANS-001 inline (default OFF; persisted via localStorage)
      - per-message expand/collapse state (expanded_message_ids Set<string>)
  - Dev harness (because no real chat surface yet):
    - Add a small section in app/src/components/DebugPanel.tsx to append sample user + assistant(frontend) entries so the timeline can be validated manually.
    - The harness MUST NOT delete or rewrite chat.jsonl; append-only only. (New sessions may be added later if needed, but not required for v1.)

- Resolved decisions:
  - Session identity is per app runtime instance (new UUID on app start; no reuse across restarts in this WP).
  - ResponseBehaviorContract TS typing is minimal (spec-aligned top-level surface + UI-used fields) while storing the full JSON object for persistence/inspection.
- Open questions:
- Notes:

## IMPLEMENTATION
- Tauri persistence: added per-session append-only JSONL log under `{APP_DATA}/sessions/<session_id>/chat.jsonl` with `turn_index` sequencing and Tauri commands for read/append/get_session_id (spec 2.10.4).
- Frontend: added TS types/helpers for ANS-001 (`ResponseBehaviorContract`) and a global "ANS-001 Timeline" drawer with default-hide inline + per-message expand and a global show-inline toggle (spec 2.7.1.7).
- Backend Flight Recorder: added runtime chat event types (FR-EVT-RUNTIME-CHAT-101..103), strict leak-safe payload validation, and a new ingestion endpoint for runtime chat events (spec 11.5.10 + EXEC-060).
- Tests: added backend tests that accept valid runtime chat payloads and reject malformed/leaky payloads.

## HYGIENE
- Commands (see ## EVIDENCE for logs):
  - CARGO_INCREMENTAL=0 cargo test --manifest-path src/backend/handshake_core/Cargo.toml --jobs 1 (exit 0; warnings present)
  - pnpm -C app test (exit 0)
  - pnpm -C app lint (exit 0)
  - pnpm -C app build (exit 0; chunk size warnings)
  - just post-work WP-1-Response-Behavior-ANS-001 (exit 0; warnings for new files not present in HEAD)

## VALIDATION
- (Mechanical manifest for audit. Records 'What' hashes/lines for Validator audit. NOT a claim of official Validation.)

### Manifest Entry 1: app/src-tauri/Cargo.toml
- **Target File**: `app/src-tauri/Cargo.toml`
- **Start**: 30
- **End**: 30
- **Line Delta**: 1
- **Pre-SHA1**: `d5c19676e2a24a418605c04cda107eeecc513f04`
- **Post-SHA1**: `c26e6675b1fba4f6b8a3efa436660df131faa126`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 2: app/src-tauri/src/lib.rs
- **Target File**: `app/src-tauri/src/lib.rs`
- **Start**: 8
- **End**: 102
- **Line Delta**: 11
- **Pre-SHA1**: `ca93c34812d5db88c5f570e3609e6574548015c5`
- **Post-SHA1**: `84f82b2af9ab377fcb2656be9b6de5d3bc205413`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 3: app/src-tauri/src/session_chat_log.rs
- **Target File**: `app/src-tauri/src/session_chat_log.rs`
- **Start**: 1
- **End**: 283
- **Line Delta**: 283
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `e0037df546362e4af86a74ca99e3b0ceffa3695b`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 4: app/src/App.css
- **Target File**: `app/src/App.css`
- **Start**: 995
- **End**: 1111
- **Line Delta**: 117
- **Pre-SHA1**: `65a2f2b004901e6492ca703fa9df8acbc2f221cd`
- **Post-SHA1**: `70ed6ee27f439ec5cb4b1c3833601b2e116a38c3`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 5: app/src/App.tsx
- **Target File**: `app/src/App.tsx`
- **Start**: 21
- **End**: 185
- **Line Delta**: 4
- **Pre-SHA1**: `4c23aa346b2c4fff3c5231c40c92e6bd3ca2a20f`
- **Post-SHA1**: `c3bf563d04d8f848c2a9d32a361c8d8a7be548be`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 6: app/src/components/DebugPanel.tsx
- **Target File**: `app/src/components/DebugPanel.tsx`
- **Start**: 1
- **End**: 260
- **Line Delta**: 164
- **Pre-SHA1**: `f02c05e478552f535e26c12581961248d55cb0f4`
- **Post-SHA1**: `f302980d6d2777d58deca1f73783155fae0d7044`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 7: app/src/components/operator/Ans001TimelineDrawer.tsx
- **Target File**: `app/src/components/operator/Ans001TimelineDrawer.tsx`
- **Start**: 1
- **End**: 188
- **Line Delta**: 188
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `2ec87c96ab3b2a83249ec294f62516b990445ea5`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 8: app/src/components/operator/index.ts
- **Target File**: `app/src/components/operator/index.ts`
- **Start**: 4
- **End**: 4
- **Line Delta**: 1
- **Pre-SHA1**: `5580c1547e919a234e8c1b89b980fa148f22b572`
- **Post-SHA1**: `c14c5ead91c51c452f7d3206b26a2f9f62d849ea`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 9: app/src/lib/ans001.ts
- **Target File**: `app/src/lib/ans001.ts`
- **Start**: 1
- **End**: 113
- **Line Delta**: 113
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `b9ef0baccd521dce33edc0a7dbfc9e98690d8457`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 10: app/src/lib/api.ts
- **Target File**: `app/src/lib/api.ts`
- **Start**: 171
- **End**: 500
- **Line Delta**: 36
- **Pre-SHA1**: `62268afad1bf2f2290028c010aeab1efd1dfb576`
- **Post-SHA1**: `45af6603fa4578d99c451f17959576a3b27edb11`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 11: app/src/lib/crypto.ts
- **Target File**: `app/src/lib/crypto.ts`
- **Start**: 1
- **End**: 29
- **Line Delta**: 29
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `33d6200874fffff65a40ff90ca77a711d101d3b0`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 12: app/src/lib/sessionChat.ts
- **Target File**: `app/src/lib/sessionChat.ts`
- **Start**: 1
- **End**: 48
- **Line Delta**: 48
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `e314d9688a8a54553a2042c77b882069b4ebdf09`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 13: src/backend/handshake_core/src/api/flight_recorder.rs
- **Target File**: `src/backend/handshake_core/src/api/flight_recorder.rs`
- **Start**: 4
- **End**: 148
- **Line Delta**: 98
- **Pre-SHA1**: `7645e2aaea62cc9cdd061f729f38806e2a47da39`
- **Post-SHA1**: `ea1346b4984a634573fbc011744b69460b2899db`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 14: src/backend/handshake_core/src/flight_recorder/duckdb.rs
- **Target File**: `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- **Start**: 714
- **End**: 722
- **Line Delta**: 9
- **Pre-SHA1**: `cdcc07e1ceb4877e7837660e57788791844bd1d3`
- **Post-SHA1**: `0d51d61a439ddace9226aec28bd01f07e9ccb035`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 15: src/backend/handshake_core/src/flight_recorder/mod.rs
- **Target File**: `src/backend/handshake_core/src/flight_recorder/mod.rs`
- **Start**: 76
- **End**: 1482
- **Line Delta**: 229
- **Pre-SHA1**: `1ae6968a8762918b4f5eac3ce0ddcf69f3a711c5`
- **Post-SHA1**: `287d23e31c1f2971ead4f672610c36cffe8cc70e`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

### Manifest Entry 16: src/backend/handshake_core/tests/runtime_chat_events_tests.rs
- **Target File**: `src/backend/handshake_core/tests/runtime_chat_events_tests.rs`
- **Start**: 1
- **End**: 114
- **Line Delta**: 114
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `56db4793eabe498e2d6837676be4e77ddefc4f3c`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage

- **Lint Results**: pnpm -C app lint
- **Artifacts**: {APP_DATA}/sessions/<session_id>/chat.jsonl (local-only; no upload in this WP)
- **Timestamp**: 2026-01-29T00:00:00Z
- **Operator**: ilja
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.121.md
- **Notes**: Pre-SHA1 from HEAD; Post-SHA1 from INDEX (staged) via `just cor701-sha`.

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (bootstrap claim)
- What changed in this update: Implemented session chat JSONL persistence + ANS-001 inspection UI + leak-safe runtime telemetry ingestion + backend payload validation tests; filled deterministic VALIDATION manifests.
- Touched files:
  - app/src-tauri/Cargo.toml
  - app/src-tauri/src/lib.rs
  - app/src-tauri/src/session_chat_log.rs
  - app/src/App.css
  - app/src/App.tsx
  - app/src/components/DebugPanel.tsx
  - app/src/components/operator/Ans001TimelineDrawer.tsx
  - app/src/components/operator/index.ts
  - app/src/lib/api.ts
  - app/src/lib/ans001.ts
  - app/src/lib/crypto.ts
  - app/src/lib/sessionChat.ts
  - src/backend/handshake_core/src/api/flight_recorder.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/tests/runtime_chat_events_tests.rs
- Next step / handoff hint: Re-run `just post-work WP-1-Response-Behavior-ANS-001`, then commit the staged implementation files on `feat/WP-1-Response-Behavior-ANS-001`.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
