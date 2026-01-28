## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Keep this file ASCII-only. Non-ASCII characters must be written as \\uXXXX escapes inside the excerpt block.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-Response-Behavior-ANS-001
- CREATED_AT: 2026-01-28
- SPEC_TARGET_RESOLVED: docs/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.121.md
- SPEC_TARGET_SHA1: 598f2e56a463e1ddf056d9f89249c5697b68c971
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja280120261944
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-Response-Behavior-ANS-001

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE. The Master Spec v02.121 main body defines the needed UI presentation rules, session chat log schema + path, and the runtime telemetry event IDs.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- Spec event IDs: FR-EVT-RUNTIME-CHAT-101..103 (\\u00A711.5.10).
- Telemetry triggers:
  - FR-EVT-RUNTIME-CHAT-101: when a user or assistant message is appended to the runtime session chat log.
  - FR-EVT-RUNTIME-CHAT-102: on ANS-001 validation attempt for frontend assistant messages (leak-safe; hashes only).
  - FR-EVT-RUNTIME-CHAT-103: when the interactive chat session is closed.
- Concurrency constraint (execution risk, not spec gap):
  - WP-1-Global-Silent-Edit-Guard is concurrently modifying Handshake Core Flight Recorder files.
  - This WP must avoid touching those same files until coder1 is validated/merged.
  - Implement UI + session chat JSONL persistence first; defer Flight Recorder wiring work if required.

### RED_TEAM_ADVISORY (security failure modes)
- Data sensitivity: the session chat log is raw (no censorship). It can contain secrets/PII. Store locally under {APP_DATA} only; do not sync or upload.
- Injection/safety: write JSONL with a safe encoder; avoid string concatenation; ensure newline handling cannot corrupt the log (no partial JSON lines).
- Export boundary: allow redaction only at explicit export boundaries; MUST NOT write back into the session chat log.
- No double-duty: the model must not be asked to maintain chat history; logging is mechanical from captured I/O.

### PRIMITIVES (traits/structs/enums)
- ResponseBehaviorContract (spec \\u00A72.7.1).
- SessionChatLogEntryV0_1, SessionChatRole (spec \\u00A72.10.4).
- RuntimeChatEventV0_1 (spec \\u00A711.5.10).
- UI state primitives (frontend): show_ans001_inline:boolean, expanded_message_ids:set<string>, ans001_timeline:selected_message_id.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [x] PASS
- Explicitly named: [x] PASS
- Specific: [x] PASS
- Measurable acceptance criteria: [x] PASS
- No ambiguity: [x] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec v02.121 provides normative requirements and a concrete schema for capture + persistence + UI inspection (\\u00A72.7.1.7, \\u00A72.10.4) and binds compliance logging + FR event IDs (\\u00A72.8.v02.13, \\u00A711.5.10).
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE.

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Spec v02.121 already contains the normative rules, schema, and acceptance-level requirements for this WP. No additional Master Spec text is needed.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED: one or more)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.121.md 2.7.1.7 Presentation + Persistence (Frontend UI) (Normative) [ADD v02.121]
- CONTEXT_START_LINE: 13410
- CONTEXT_END_LINE: 13429
- CONTEXT_TOKEN: ANS-001 Timeline
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.7.1.7 Presentation + Persistence (Frontend UI) (Normative) [ADD v02.121]

  MUST:
  - For runtime model role `frontend` (\\u00A74.3.3.4.2), every assistant message MUST have an ANS-001 payload.
  - Chat surfaces MUST default to hiding the ANS-001 payload inline.
  - Chat surfaces MUST provide per-message expand/collapse and a global show-inline toggle (default OFF).
  - Persistence MUST be mechanical (runtime-owned) and MUST NOT require the model to restate prior chat history (see \\u00A72.10.4).

  SHOULD:
  - Provide a side panel "ANS-001 Timeline" that lists messages newest->oldest and allows opening the full ANS-001 payload.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.121.md 2.10.4 Frontend Session Chat Log (ANS-001) (Normative) [ADD v02.121]
- CONTEXT_START_LINE: 16026
- CONTEXT_END_LINE: 16087
- CONTEXT_TOKEN: chat.jsonl
- EXCERPT_ASCII_ESCAPED:
  ```text
  ### 2.10.4 Frontend Session Chat Log (ANS-001) (Normative) [ADD v02.121]

  Storage location (runtime, not repo)
  - The runtime MUST store the session chat log under the application data root `{APP_DATA}`.
  - One file per session:
    - `{APP_DATA}/sessions/<session_id>/chat.jsonl`

  File format
  - JSON Lines (one JSON object per line).
  - Append-only; no in-place edits.

  Schema (normative)
  - schema_version: "hsk.session_chat_log@0.1"
  - ans001 is present only for role="assistant" and model_role="frontend".
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.121.md 2.8.v02.13 ANS-001 Invocation (EXEC-057 to EXEC-060)
- CONTEXT_START_LINE: 14965
- CONTEXT_END_LINE: 15001
- CONTEXT_TOKEN: EXEC-060
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 2.8.v02.13 ANS-001 Invocation (EXEC-057 to EXEC-060)

  EXEC-060 (Normative): Compliance logging
  - On every ANS-001 validation attempt, the runtime MUST emit a leak-safe Flight Recorder event (FR-EVT-RUNTIME-CHAT-102, \\u00A711.5.10) containing ans001_compliant and violation_clauses (hashes only; no inline content).
  - For interactive chat sessions with runtime model role `frontend`, the runtime MUST append a SessionChatLogEntryV0_1 record (\\u00A72.10.4) for each user/assistant message, including ANS-001 payloads for assistant messages.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.121.md 11.5.10 FR-EVT-RUNTIME-CHAT-101..103 (Frontend Conversation + ANS-001 Telemetry) (Normative) [ADD v02.121]
- CONTEXT_START_LINE: 53491
- CONTEXT_END_LINE: 53540
- CONTEXT_TOKEN: FR-EVT-RUNTIME-CHAT-102
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 11.5.10 FR-EVT-RUNTIME-CHAT-101..103 (Frontend Conversation + ANS-001 Telemetry) (Normative) [ADD v02.121]

  - FR-EVT-RUNTIME-CHAT-101 - message appended (user or assistant)
  - FR-EVT-RUNTIME-CHAT-102 - ANS-001 validation result (frontend assistant messages)
  - FR-EVT-RUNTIME-CHAT-103 - session closed

  Rules:
  - Flight Recorder MUST NOT store inline chat message bodies; only hashes/pointers.
  - For runtime model role `frontend` assistant messages, the runtime MUST emit FR-EVT-RUNTIME-CHAT-102 with ans001_compliant and violation_clauses.
  ```
