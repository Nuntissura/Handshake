# Orchestrator Bundle

This README is navigational only.
Authoritative folder-placement law for the Orchestrator bundle lives in `.GOV/codex/Handshake_Codex_v1.4.md` plus `ORCHESTRATOR_PROTOCOL.md`.

## Primary Docs

- `ORCHESTRATOR_PROTOCOL.md`

## Archived Support Docs

- `../../reference/legacy/orchestrator/README.md`

## Current Runtime State

- `runtime/ORCHESTRATOR_GATES.json`
  - live orchestrator runtime state
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

## Role Map

- `runtime/`
  - orchestrator-owned machine state only
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
- external repo-governance `roles_shared/ROLE_SESSION_REGISTRY.json`
- external repo-governance `roles_shared/WP_COMMUNICATIONS/`
- `.GOV/roles_shared/records/TASK_BOARD.md`
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`

## Key Commands

- `just orchestrator-startup`
- `just orchestrator-next [WP-{ID}]`
- `just record-refinement WP-{ID}`
- `just record-signature WP-{ID} ...`
- `just record-prepare WP-{ID} ...`
- `just orchestrator-prepare-and-packet WP-{ID}`
- `just launch-coder-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-wp-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-integration-validator-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
