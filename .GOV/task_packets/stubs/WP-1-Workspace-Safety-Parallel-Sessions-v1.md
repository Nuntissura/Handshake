# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Workspace-Safety-Parallel-Sessions-v1

## STUB_METADATA
- WP_ID: WP-1-Workspace-Safety-Parallel-Sessions-v1
- BASE_WP_ID: WP-1-Workspace-Safety-Parallel-Sessions
- CREATED_AT: 2026-02-24T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.137.md 7.6.3 (Phase 1) -> item 33 (workspace safety boundaries for parallel writes)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.137.md 4.3.9.17 Workspace Safety Boundaries for Parallel Sessions (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.2.4 Work Unit lock contract (file-scope locks / IN_SCOPE_PATHS)
  - Handshake_Master_Spec_v02.137.md 6.0.2 Unified Tool Surface Contract -> Tool Gate (command denylist enforcement)

## INTENT (DRAFT)
- What: Enforce workspace safety boundaries for parallel sessions: worktree isolation and/or strict file-scope locks, deny-by-default cross-session access, command denylist, and merge-back discipline.
- Why: Parallel sessions touching the same repo/workspace can silently overwrite or conflict without deterministic isolation and fail-closed execution rules.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Isolation strategy:
    - Support worktree isolation (preferred) and/or file-scope lock isolation (fallback).
  - Enforcement:
    - INV-WS-001 in-scope path enforcement (no writes outside declared IN_SCOPE_PATHS).
    - INV-WS-003 deny cross-session access to another session’s uncommitted worktree by default.
    - Command denylist enforced by Tool Gate for spawned/background sessions (destructive git ops, rm -rf outside scope, and governance artifact modification bypass).
  - Merge-back discipline:
    - Session produces merge-ready diff/patch artifact with provenance.
    - Merge-back is explicit operator-governed action with Flight Recorder logging; conflicts surface as BLOCKED with explicit conflict report.
- OUT_OF_SCOPE:
  - Non-git worksurface isolation (Design Studio entity locking) (Phase 2+).

## ACCEPTANCE_CRITERIA (DRAFT)
- With 2 sessions on the same workspace, overlapping writes are deterministically prevented (worktree isolation or lock block) with explicit errors and evidence.
- Spawned/background sessions cannot run denied destructive commands without per-invocation approval.
- Merge-back is explicit and produces Flight Recorder evidence; merge conflicts are not silently resolved.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Session spawn (WP-1-Session-Spawn-Contract-v1) and session scheduler baseline (WP-1-ModelSession-Core-Scheduler-v1).
- Coordinates with: existing worktree concurrency processes and Codex destructive-op constraints (no hidden git rewrites).

## RISKS / UNKNOWNs (DRAFT)
- Risk: incomplete scope derivation for sessions/MTs makes enforcement noisy; may require explicit scoping UX/workflow in DCC.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Workspace-Safety-Parallel-Sessions-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Workspace-Safety-Parallel-Sessions-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

