# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Dev-Command-Center-MVP-v1

## STUB_METADATA
- WP_ID: WP-1-Dev-Command-Center-MVP-v1
- BASE_WP_ID: WP-1-Dev-Command-Center-MVP
- CREATED_AT: 2026-02-18T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.131.md 7.6.3 (Phase 1) -> [ADD v02.127 - Dev Command Center (DCC) MVP (Sidecar-derived)]
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.131.md 10.11 Dev Command Center (Sidecar Integration) [ADD v02.127]
  - Handshake_Master_Spec_v02.131.md 10.11.5 Handshake feature set (Sidecar-derived + enriched)
  - Handshake_Master_Spec_v02.131.md 10.11.7 Data model (expanded + storage)
  - Handshake_Master_Spec_v02.131.md 10.11.8 Governance, capabilities, and safety
  - Handshake_Master_Spec_v02.131.md 10.11.A .handshake/workspace.json schema (v1.0) + migration rules
  - Handshake_Master_Spec_v02.131.md 10.11.B devcc.db SQLite schema (v1) DDL
  - Handshake_Master_Spec_v02.131.md 10.11.D Worktree lifecycle (job wrapper guidance)
  - Handshake_Master_Spec_v02.137.md 10.11.X Multi-Session Steering Panel [ADD v02.137]

## INTENT (DRAFT)
- What: Deliver the Phase 1 Dev Command Center (DCC) MVP: a canonical control surface that binds Locus work (WP/MT) <-> git workspaces (worktrees) <-> execution sessions <-> approvals/logs/diffs, without bypassing Workflow Engine, gates, or Flight Recorder.
- Why: Make governed development usable (approvals as inbox, VCS as panel, predictable traceable operations) while enabling safe parallel work and deterministic recovery/handoffs.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - DCC UI shell (kanban-first) with panels: Work Packets, Workspaces (worktrees), Sessions, Approvals, VCS (status/diff), Search, and a minimal Timeline.
  - Workspace registry:
    - list/add/open/close worktrees
    - import existing worktrees
    - link workspace <-> wp_id/mt_id (Locus remains authoritative)
  - Worktree management as governed jobs/wrappers:
    - create/open/prune with safe defaults
    - destructive git worktree rewrites (reset/clean/rebase) require explicit same-turn approval (Codex CX-108)
  - Execution Session Manager:
    - show active sessions (role/model/backend), workspace binding, and capability grants
    - deep-link to Job History + Flight Recorder timeline slices
    - multi-session steering: session list + state machine + spawn tree + per-session budgets and controls (pause/resume/cancel)
  - Approval Inbox:
    - collect pending approvals from capability gate / Workflow Engine
    - approve-once / approve-for-job / approve-for-workspace / deny (+ reason)
    - log all decisions to Flight Recorder
    - denied approvals deterministically block the job with explicit failure code and no partial side effects
  - VCS review loop:
    - show version.status + version.diff
    - commit flow uses version.commit(paths[]) with commit message captured as an artifact (no implicit staging)
    - dangerous ops require same-turn explicit approval (no silent reset/clean/rebase)
  - Objective Anchor Store (minimal):
    - create/view anchors and handoff records linked to wp_id/mt_id
    - append-only notes; non-authoritative; MUST NOT override Locus status
  - Storage foundation:
    - ship `.handshake/workspace.json` schema v1.0 and `devcc.db` schema v1 (+ migrations)
    - MUST be local-first, contain no secrets, and be safe to delete/rebuild from repo state + Locus
    - default ignore patterns documented (gitignore posture)
  - Conversation timeline (Phase A baseline):
    - ingest Handshake-native conversations/events (roles + Flight Recorder) into a DCC timeline
    - adapter contract stubbed for later external sources
- OUT_OF_SCOPE:
  - GitHub PR/comment sync.
  - Multi-user workspace sync / shared approvals.
  - Full external conversation ingestion (beyond adapter skeleton + at least one pilot, if any).
  - Any UI commitment to Sidecar keybindings/TUI parity.
  - Any ungoverned direct tool execution from UI (hard no-bypass).

## ACCEPTANCE_CRITERIA (DRAFT)
- Operator can open DCC, select a WP, open its linked worktree, view diff, approve a needed capability, and run a governed commit without leaving Handshake.
- Every stateful DCC action emits Flight Recorder events and is traceable to wp_id/mt_id/session_id/wsid.
- Denied approvals block deterministically with an explicit failure code and no partial side effects.
- `.handshake/workspace.json` and `devcc.db` can be deleted and rebuilt from repo state + Locus without corrupting canonical governance artifacts.
- DCC kanban lanes for WP statuses (from Locus) exist, with deep links to worktree, sessions, approvals, and Flight Recorder slices.
- Approvals UX exists as a single compact list with previews and scoping (once/job/workspace).
- Vertical slice (Phase 1 roadmap): run one WP end-to-end using DCC:
  - create/link worktree -> run job -> approval prompt -> review diff -> commit -> mark MT done
  - confirm Task Board sync and Flight Recorder evidence.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on Locus WP/MT plumbing + Task Board sync primitives (see existing Locus stub: WP-1-Locus-Work-Tracking-System-Phase1-v1).
- Depends on capability registry + capability gate approval request plumbing (SSoT + gate UX bridging).
- Depends on engine.version (status/diff/commit) and Mechanical Extension v1.2 no-bypass envelopes/gates.
- Depends on Flight Recorder + Job History/Operator Consoles deep-linking surfaces to make DCC actions auditable.

## RISKS / UNKNOWNs (DRAFT)
- Risk: authority drift (DCC becoming the source of truth). Mitigation: Locus remains canonical; DCC stores only references + non-authoritative anchors/handoffs.
- Risk: unsafe git operations exposed via UI. Mitigation: enforce explicit same-turn approval for reset/clean/rebase; no implicit staging; artifact-first commit messages.
- Risk: approval scoping semantics (once/job/workspace) and caching/TTL may be under-specified for MVP; must align with capability system rules.
- Risk: rebuild semantics for devcc.db/workspace.json could leak secrets or diverge across platforms if not carefully specified and validated.
- Risk: DCC timeline ingestion can balloon in scope; Phase 1 should restrict to Handshake-native events and an adapter stub.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Dev-Command-Center-MVP-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Dev-Command-Center-MVP-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
