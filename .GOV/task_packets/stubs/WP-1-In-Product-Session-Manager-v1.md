# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-In-Product-Session-Manager-v1

## STUB_METADATA
- WP_ID: WP-1-In-Product-Session-Manager-v1
- BASE_WP_ID: WP-1-In-Product-Session-Manager
- CREATED_AT: 2026-04-06T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Dev-Command-Center-Control-Plane-Backend
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Replace OS terminal windows with in-app DCC session display
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md 10.11 Dev Command Center (Sidecar Integration)
  - Handshake_Master_Spec_v02.179.md 10.11.5.4 Execution Session Manager
- DISCOVERY_ORIGIN: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION (RGF-107, AgentsRoom pattern)

## INTENT (DRAFT)
- What: Replace OS terminal windows with an in-app DCC session panel showing live model interactions, command outputs, governance state, and session status indicators (thinking, coding, done, blocked, error). Operators inspect and steer ongoing work without opening OS terminals.
- Why: Based on AgentsRoom real-time monitoring pattern and industry trend toward "AI as the development platform." Current workflow requires switching between multiple OS terminal windows to monitor governed sessions. An in-app session panel centralizes monitoring, reduces context-switching, and enables richer interaction (status badges, filtering, one-click controls) than raw terminal output.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - DCC session panel with live model interaction stream per active session.
  - Session status indicators (READY, RUNNING, FAILED, COMPLETED) with visual badges.
  - MT progress display per session (task board visualization).
  - Inter-session message flow display (who sent what to whom via auto-relay).
  - One-click session restart, cancel, and redirect controls.
  - Session output filtering and search.
  - Integration with existing ACP broker session registry.
- OUT_OF_SCOPE:
  - The ACP broker itself (already exists).
  - Terminal window management (addressed by RGF-95).
  - Visual debugging / screenshot comparison (WP-1-Visual-Debugging-Loop-v1).

## ACCEPTANCE_CRITERIA (DRAFT)
- The DCC session panel displays all active governed sessions with real-time status updates.
- Session status indicators visually reflect current state (READY, RUNNING, FAILED, COMPLETED).
- MT progress is visible per session, showing task board state inline.
- Inter-session message flow (auto-relay) is displayed with sender, receiver, and payload summary.
- Operators can restart, cancel, or redirect a session via one-click controls without opening OS terminals.
- Session output supports filtering by severity, session ID, and free-text search.
- The panel integrates with the existing ACP broker session registry as the single source of truth.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on WP-1-Dev-Command-Center-Control-Plane-Backend for the DCC backend infrastructure.
- Requires the ACP broker session registry to be accessible via API.
- No spec blockers; spec anchors exist in 10.11 and 10.11.5.4.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Real-time streaming of model interaction output to the frontend may introduce latency or memory pressure for long-running sessions.
- Risk: Session control actions (restart, cancel, redirect) require robust state machine transitions to avoid orphaning sessions or double-starting.
- Risk: Cross-boundary WP (backend + frontend) increases coordination complexity during implementation.
- Unknown: Whether WebSocket or SSE is the preferred transport for live session streaming in the Tauri + React stack.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-In-Product-Session-Manager-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-In-Product-Session-Manager-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
