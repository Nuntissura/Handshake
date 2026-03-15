# Orchestrator Bundle

## Primary Docs

- `ORCHESTRATOR_PROTOCOL.md`
- `docs/ORCHESTRATOR_RUBRIC.md`
- `docs/ORCHESTRATOR_IMPLEMENTATION_ROADMAP.md`
- `docs/ORCHESTRATOR_PRIORITIES.md`
- `docs/ORCHESTRATOR_PROTOCOL_GAPS.md`

## Current Runtime State

- `ORCHESTRATOR_GATES.json`
  - live legacy root state still consumed by current tooling; new orchestrator-owned state belongs under `runtime/`
- `checks/orchestrator_gates.mjs`
- `scripts/create-task-packet.mjs`
- `scripts/create-task-packet-stub.mjs`
- `scripts/orchestrator-next.mjs`
- `scripts/launch-cli-session.mjs`
- `scripts/session-control-broker.mjs`
- `scripts/session-control-command.mjs`
- `scripts/session-control-cancel.mjs`
- `scripts/session-registry-status.mjs`
- `scripts/operator-monitor-tui.mjs`
- `scripts/task-board-set.mjs`
- `scripts/wp-traceability-set.mjs`

## Role Layout

- `runtime/`
  - orchestrator-owned machine state only; new state belongs here even while `ORCHESTRATOR_GATES.json` remains as migration residue
- `scripts/`
  - orchestrator-owned entrypoints
- `scripts/lib/`
  - orchestrator-only helper libraries
- `checks/`
  - orchestrator-owned enforcement/gate entrypoints
- `tests/`
  - governance tests for orchestrator scripts/checks
- `fixtures/`
  - orchestrator-local test data and golden inputs

## Shared Dependencies To Know

- `.GOV/roles_shared/checks/README.md`
- `.GOV/roles_shared/scripts/README.md`
- `.GOV/roles_shared/runtime/ROLE_SESSION_REGISTRY.json`
- `.GOV/roles_shared/runtime/WP_COMMUNICATIONS/`
- `.GOV/roles_shared/records/TASK_BOARD.md`
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`

## Key Commands

- `just orchestrator-startup`
- `just orchestrator-next [WP-{ID}]`
- `just record-refinement WP-{ID}`
- `just record-signature WP-{ID} ...`
- `just record-prepare WP-{ID} ...`
- `just orchestrator-prepare-and-packet WP-{ID}`
- `just launch-coder-session WP-{ID}`
- `just launch-wp-validator-session WP-{ID}`
- `just launch-integration-validator-session WP-{ID}`
