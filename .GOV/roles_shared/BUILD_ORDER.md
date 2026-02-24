# Build Order (Advisory) [CX-BO-001]

This file provides a RECOMMENDED build order for Phase 1 Work Packets (WPs).

- This build order is NOT binding: the Master Spec + active task packets define "Done".
- File upkeep IS binding: the Orchestrator MUST keep this file current (see Orchestrator Protocol).

## Source of truth (do not contradict)

- Master Spec: `.GOV/roles_shared/SPEC_CURRENT.md` (resolves to a versioned `Handshake_Master_Spec_vXX.XXX.md`)
- Task Board status: `.GOV/roles_shared/TASK_BOARD.md`
- Base->Active packet mapping: `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`
- Exact scope/acceptance: the active task packet under `.GOV/task_packets/`

## Update triggers (Orchestrator MUST update)

Update this file when any of the following occurs:
- A stub becomes activated (new official packet created) or a new `-vN` revision becomes the active packet.
- A new blocking dependency is discovered that changes the recommended sequencing.
- SPEC_CURRENT changes in a way that adds/removes/reshapes Phase 1 deliverables.
- A WP becomes "blocking" for a target outcome (e.g., IDE replacement, parallel cloud sessions).

## How to use this file

1. Use this as the default sequencing suggestion when creating/activating WPs.
2. Record concrete dependency edges in:
   - the task packet `## Dependencies`, and
   - `.GOV/roles_shared/TASK_BOARD.md` (blocker lines).
3. If this file and TASK_BOARD disagree, reconcile them in the same governance maintenance pass.

## Recommended sequencing (Phase 1)

### 0) Contract + no-bypass enforcement (Phase 1 blocker)

1. WP-1-Unified-Tool-Surface-Contract-v1 (STUB)
  - Rationale: per spec [ADD v02.136], local tool calling and MCP MUST use the same Tool Registry + Tool Gate + Flight Recorder event model (no bypass).

### 1) Safety baseline for "Handshake as IDE" (hardening follow-ups)

MEX baseline is already VALIDATED (see TASK_BOARD). These remediation packets remain as hardening follow-ups:

1. WP-1-MEX-Safety-Gates-v2 (STUB)
2. WP-1-MEX-Observability-v2 (STUB)
3. WP-1-MEX-UX-Bridges-v2 (STUB)

### 2) Model routing + governance prerequisites (before parallel cloud sessions)

These enable "who can call what model/provider, under which policy" with auditability:

1. WP-1-Model-Profiles-v2 (STUB)
2. WP-1-Work-Profiles-v1 (STUB)
3. WP-1-Inbox-Role-Mailbox-Alignment-v1 (STUB)

### 3) Parallelism foundations (runtime enforcement)

- WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1 (STUB)
  - Rationale: file-scope locks, lifecycle telemetry, execution identity, deterministic blocking codes.

### 4) "Control room" surfaces (IDE replacement path)

These are the planned Operator-facing coordination surfaces:

1. WP-1-Locus-Work-Tracking-System-Phase1-v1 (STUB)
2. WP-1-Dev-Command-Center-MVP-v1 (STUB)

### 5) Web capture + media browsing/search (after ingestion + control surfaces)

1. WP-1-Handshake-Stage-MVP-v1 (STUB)
2. WP-1-Video-Archive-Loom-Integration-v1 (STUB)

### 9) Already VALIDATED (do not schedule again)

- WP-1-Cloud-Escalation-Consent-v2
- WP-1-Loom-MVP-v1
- WP-1-Media-Downloader-v2
- WP-1-MEX-v1.2-Runtime-v3
- WP-1-Supply-Chain-MEX-v2

## Notes / constraints

- Concurrency: do not run overlapping WPs that touch the same IN_SCOPE_PATHS; use separate worktrees per WP.
- Governance: cloud use requires explicit consent artifacts; do not "parallelize" by bypassing receipts/policy.
- This file should stay compact; details belong in task packets and TASK_BOARD blocker lines.
