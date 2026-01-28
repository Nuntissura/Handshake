# Work Packet Stub: WP-1-Response-Behavior-ANS-001

## STUB_METADATA
- WP_ID: WP-1-Response-Behavior-ANS-001
- CREATED_AT: 2026-01-08
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: docs/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.121.md 7.6.3 item 25 ([ADD v02.103] Response Behavior Contract (Diary ANS-001) + [ADD v02.121] frontend session chat log + UI toggles)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.121.md 2.7 Response Behavior Contract (Diary ANS-001)
  - Handshake_Master_Spec_v02.121.md 2.7.1 The Behavior Contract
  - Handshake_Master_Spec_v02.121.md 2.7.1.7 Presentation + Persistence (Frontend UI) (Normative) [ADD v02.121]
  - Handshake_Master_Spec_v02.121.md 2.8.v02.13 ANS-001 Invocation (EXEC-057 to EXEC-060) (EXEC-060 logging semantics) [ADD v02.121]
  - Handshake_Master_Spec_v02.121.md 2.10.4 Frontend Session Chat Log (ANS-001) (Normative) [ADD v02.121]
  - Handshake_Master_Spec_v02.121.md 11.5.10 FR-EVT-RUNTIME-CHAT-101..103 (Frontend Conversation + ANS-001 Telemetry) (Normative) [ADD v02.121]

## INTENT (DRAFT)
- What: Implement the governed assistant response behavior contract (ANS-001) for the `frontend` runtime model role, including capture, mechanical persistence, and basic UI inspection controls.
- Why: Keeps interactive chat trustworthy and inspectable without relying on chat-as-state. Also enables fast operator inspection/debugging with a per-session chat log that includes the ANS-001 payload.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define and emit ResponseBehaviorContract fields (Answer, IntentConfirmation, ModeContext, OperationPlan, ProactiveSurfacing, NextSteps) for `frontend` assistant messages.
  - Enforce WorkMode behavior constraints (determinism required except Brainstorm; allowed operations differ per mode).
  - Ensure operation planning is shown before execution when edits/ops are possible (abortable/reversible flags surfaced).
  - Provide deterministic validation for the contract shape (schema + tests) so missing fields are treated as violations.
  - Persist interactive chat sessions mechanically to `{APP_DATA}/sessions/<session_id>/chat.jsonl` (append-only JSONL), including raw chat content and the ANS-001 payload for frontend assistant messages.
  - UI: ANS-001 hidden inline by default with per-message expand + global show-inline toggle, and a side-panel ANS-001 timeline viewer (newest->oldest).
  - Emit leak-safe runtime telemetry events for chat + ANS-001 validation (`FR-EVT-RUNTIME-CHAT-101..103`).
- OUT_OF_SCOPE:
  - Full UX polish of every surface (initially implement in the primary assistant surface and add adapters later).
  - Extending work modes beyond those defined in ANS-001.
  - Long-term "work surface" UI (Kanban, project-wide trackers, agent/model overview panels) beyond the basic ANS-001 panel/toggles.

## ACCEPTANCE_CRITERIA (DRAFT)
- In governed modes, every assistant response includes: direct answer, intent confirmation, mode context, proactive surfacing, and next steps (operation plan required when performing edits/ops).
- Brainstorm mode can omit determinism-only fields but must still respect the contract’s allowed-operations constraints.
- Violations are detectable deterministically (unit tests + schema validation) and are visible as structured diagnostics in Operator Consoles.
- For interactive chat sessions with runtime model role `frontend`, every user/assistant message is appended to `{APP_DATA}/sessions/<session_id>/chat.jsonl` (one JSON object per line), including ANS-001 payloads for frontend assistant messages.
- Default UI: ANS-001 hidden inline with per-message expand/collapse, a global show-inline toggle, and a side-panel timeline list (newest->oldest) that opens the full ANS-001 payload.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Governance mode selection must be available (STRICT/FREE/etc), including any Spec Router policy bindings if used for mode selection.
- A structured diagnostics path exists for surfacing contract violations (Operator Consoles Problems/Evidence).

## RISKS / UNKNOWNs (DRAFT)
- Risk of scope overlap with Spec Router/governance session log; activation should confirm boundary between “mode selection” and “response contract enforcement”.
- Multiple assistant entrypoints may exist (UI, CLI, MCP); must avoid partial adoption that reintroduces ungoverned responses.
- Disk growth/retention for raw per-session chat logs; ensure append is atomic and that export/redaction boundaries are explicit (no silent write-back).

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm authoritative SPEC_ANCHOR set (Main Body sections above; not Roadmap).
- [ ] Produce in-chat Technical Refinement Block (per `docs/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `docs/refinements/WP-1-Response-Behavior-ANS-001.md` (approved/signed).
- [ ] Create official task packet via `just create-task-packet WP-1-Response-Behavior-ANS-001` (in `docs/task_packets/`).
- [ ] Move Task Board entry out of STUB into Ready for Dev (In Progress is handled via Validator status-sync when work begins).
