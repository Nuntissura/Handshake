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
    RECEIPTS.jsonl
```

File roles:
- `THREAD.md`: append-only freeform talk for Operator, Orchestrator, Coder, and Validator.
- `RUNTIME_STATUS.json`: non-authoritative liveness state, waiting state, next actor, validator trigger, heartbeat posture, and bounded review-loop counters.
- `RECEIPTS.jsonl`: append-only structured receipts. Each line is one JSON object that records assignment, status, heartbeat, steering, validation, repair, or handoff activity.

Usage rules:
- New official packets should point to their per-WP communication directory in packet metadata.
- The same artifact model is used for both `MANUAL_RELAY` and `ORCHESTRATOR_MANAGED` workflow lanes.
- The packet-declared `WP_COMMUNICATION_DIR` is the only communication authority for that WP. Role-local worktrees and backup branches are never the communication authority.
- `THREAD.md` is for discussion, steering, clarifications, and soft coordination.
- `RUNTIME_STATUS.json` is for watch state and wake-up conditions; it is not packet truth.
- `RECEIPTS.jsonl` is the deterministic audit trail for session receipts. It is machine-readable and must validate against `/.GOV/schemas/WP_RECEIPT.schema.json`.
- Validation verdicts still belong only in the packet `## VALIDATION_REPORTS` section.

Runtime vocabulary:
- `runtime_status` uses the repo-governance liveness states `submitted | working | input_required | completed | failed | canceled`.
- `validator_trigger` is the validator wake-up switch and must be one of:
  - `NONE`
  - `READY_FOR_VALIDATION`
  - `VALIDATOR_QUERY`
  - `POST_WORK_PASS`
  - `BLOCKED_NEEDS_VALIDATOR`
  - `STALE_HEARTBEAT`
  - `HANDOFF_READY`

Heartbeat and wake-up rules:
- Do not poll continuously.
- Update `RUNTIME_STATUS.json` and append a receipt on session start, major phase change, blocker/unblock, handoff, completion, and every configured heartbeat interval only while actively working.
- Validator work is event-driven first. The Validator should check when `validator_trigger != NONE`, when the packet explicitly requests validation, or when the WP is assigned for validation.

Authority split:
- Packet = contract truth.
- `WORKFLOW_AUTHORITY` = Orchestrator workflow and hard-gate authority.
- `TECHNICAL_ADVISOR` = WP-scoped advisory validator.
- `TECHNICAL_AUTHORITY` = Integration Validator final technical authority.
- `MERGE_AUTHORITY` = Integration Validator final merge authority.
- `THREAD.md` = freeform collaboration.
- `RUNTIME_STATUS.json` = liveness and next-actor watch state.
- `RECEIPTS.jsonl` = deterministic receipt ledger.
- If any of those disagree, the packet wins.

Deterministic helpers:
- `just ensure-wp-communications WP-{ID}`
- `just wp-communications-check`
- `just wp-heartbeat WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <CURRENT_PHASE> <RUNTIME_STATUS> <NEXT_EXPECTED_ACTOR> "<WAITING_ON>" [VALIDATOR_TRIGGER] [LAST_EVENT]`
- `just wp-receipt-append WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <RECEIPT_KIND> "<SUMMARY>" [STATE_BEFORE] [STATE_AFTER]`
