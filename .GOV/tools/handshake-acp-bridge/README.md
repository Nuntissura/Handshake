# Handshake ACP Bridge

Governance-owned ACP adapter for Handshake role sessions.

## Purpose

This tool provides an ACP-style session surface for repo-governed role work without moving authority out of the repo.

It is the control transport layer. It is not the policy layer.

## What It Does

- runs as a persistent local broker process for governed ACP-style session control
- exposes ACP-style methods to the governed wrapper client over local JSON-line RPC
- bridges governed session prompts to Codex execution
- streams `session/update` notifications during a run
- supports stable governed session identity keyed to role plus WP
- owns active-run tracking, timeout settlement, and cancellation delivery
- leaves packets, traceability, task board, and WP communications authoritative

## Supported Methods

- `initialize`
- `session/new`
- `session/load`
- `session/prompt`
- `session/cancel`
- `session/close`
- `broker/shutdown`

## Governance Rule

This adapter must be called through the governed `just` entrypoints and the role/shared wrappers under `.GOV/roles/orchestrator/scripts/` and `.GOV/roles_shared/scripts/session/` for normal Handshake operation. Direct ACP use is a tooling/debug surface only, not an authority bypass. The broker requires a repo-scoped auth token plus `ORCHESTRATOR` / `role_orchestrator` initialization claims, and repo projections remain the authoritative audit trail.

The broker itself is not authoritative. Its runtime state is mirrored into:

- `.GOV/roles_shared/SESSION_CONTROL_REQUESTS.jsonl`
- `.GOV/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
- `.GOV/roles_shared/SESSION_CONTROL_OUTPUTS/`
- `.GOV/roles_shared/ROLE_SESSION_REGISTRY.json`
- `.GOV/roles_shared/SESSION_CONTROL_BROKER_STATE.json`

The broker state also carries build/auth identity so the governed client can refuse stale broker instances after governance changes.

## Relationship To The VS Code Bridge

The VS Code bridge is no longer the primary session control plane. It is a launch/bootstrap and notification helper.

This ACP bridge is the primary control transport for governed session steering.

One governed role/WP session may have at most one active broker-owned run at a time.

Operational note:

- `session/cancel` stops an active governed run.
- `session/close` clears the steerable thread registration for a governed role/WP session and leaves an auditable closed record in the registry.
- `broker/shutdown` only stops the broker process. It does not delete registry history or role-session records.
