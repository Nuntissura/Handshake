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
- What: Implement ANS-001 (Response Behavior Contract) capture + persistence for the `frontend` runtime model role, plus basic UI inspection (hide/show inline + side-panel timeline viewer).
- Why: Provide deterministic operator inspection/debugging of the frontend model without relying on chat-as-state or consuming tokens to restate chat history.
- IN_SCOPE_PATHS:
  - app/src/**
  - app/src-tauri/**
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

just post-work WP-1-Response-Behavior-ANS-001
```

### DONE_MEANS
- For interactive chat sessions with runtime model role `frontend`, the runtime appends one JSON object per line to `{APP_DATA}/sessions/<session_id>/chat.jsonl` (append-only, crash-safe, no in-place edits) per spec `Handshake_Master_Spec_v02.121.md` \u00A72.10.4.
- Entries include `schema_version: "hsk.session_chat_log@0.1"` and are ordered by `(turn_index, created_at_utc, message_id)` ascending.
- For assistant messages with `model_role="frontend"`, the log entry includes `ans001` payload (or `null` only when response emission was blocked) and optional `ans001_validation` (leak-safe; no full content duplication).
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
  - app/src/App.tsx
  - app/src/components/DebugPanel.tsx
  - app/src/components/operator/TimelineView.tsx
  - app/src-tauri/src/main.rs
  - app/src-tauri/src/lib.rs
- SEARCH_TERMS:
  - "ANS-001"
  - "session"
  - "appDataDir"
  - "Timeline"
  - "Flight Recorder"
- RUN_COMMANDS:
  ```bash
  pnpm -C app test
  pnpm -C app lint
  ```
- RISK_MAP:
  - "raw session chat log contains secrets" -> "ensure local-only storage under {APP_DATA}; no auto-upload"
  - "log growth / retention" -> "disk usage; rotation/retention policy deferred (out of scope for this WP)"

## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## IMPLEMENTATION
- (Coder fills after skeleton approval.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`docs/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha path/to/file` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `path/to/file`
- **Start**: <line>
- **End**: <line>
- **Line Delta**: <adds - dels>
- **Pre-SHA1**: `<hash>`
- **Post-SHA1**: `<hash>`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:

## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict.)
- Current WP_STATUS: In Progress (bootstrap claim)
- What changed in this update: Claimed CODER_MODEL/CODER_REASONING_STRENGTH and set Status to In Progress.
- Next step / handoff hint: Proceed to SKELETON phase for ANS-001 logging (Tauri commands + TS types) and request SKELETON APPROVED before implementation.

## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
