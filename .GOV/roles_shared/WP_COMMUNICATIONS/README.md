# WP_COMMUNICATIONS

This directory holds per-WP communication and liveness artifacts.

Hard rules:
- The task packet remains the authoritative contract.
- Files in `WP_COMMUNICATIONS/` are append-only collaboration and runtime-status helpers.
- If a communication artifact conflicts with the task packet, the task packet wins.
- These artifacts must not redefine scope, verdict, packet status, PREPARE assignment, or validation authority.

Per-WP layout:

```text
.GOV/roles_shared/WP_COMMUNICATIONS/
  WP-.../
    THREAD.md
    RUNTIME_STATUS.json
    RECEIPTS.md
```

File roles:
- `THREAD.md`: append-only freeform talk for Operator, Orchestrator, Coder, and Validator.
- `RUNTIME_STATUS.json`: non-authoritative liveness state, waiting state, next actor, and heartbeat posture.
- `RECEIPTS.md`: deterministic assignment, status, handoff, and heartbeat receipts.

Usage rules:
- New official packets should point to their per-WP communication directory in packet metadata.
- `THREAD.md` is for discussion, steering, clarifications, and soft coordination.
- `RUNTIME_STATUS.json` is for watch state and wake-up conditions; it is not packet truth.
- `RECEIPTS.md` is for structured breadcrumbs that help multi-session work stay deterministic.
- Validation verdicts still belong only in the packet `## VALIDATION_REPORTS` section.
