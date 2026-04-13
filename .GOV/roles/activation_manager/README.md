# Activation Manager Bundle

This README is navigational only.
Authoritative folder-placement law for the Activation Manager bundle lives in `.GOV/codex/Handshake_Codex_v1.4.md` plus `ACTIVATION_MANAGER_PROTOCOL.md`.

## Primary Live Docs

- `ACTIVATION_MANAGER_PROTOCOL.md`

## Role Purpose

- bounded pre-launch governance authoring for refinement, spec enrichment, signature recording, packet hydration, microtask preparation, worktree preparation, and activation-readiness review

## Migration Status

- governed session-control support now exists for orchestrator-managed pre-launch work:
  - `just launch-activation-manager-session WP-{ID}`
  - `just start-activation-manager-session WP-{ID}`
  - `just steer-activation-manager-session WP-{ID} "<prompt>"`
  - `just cancel-activation-manager-session WP-{ID}`
  - `just close-activation-manager-session WP-{ID}`
- the role-local action surface now stays under one recipe: `just activation-manager <startup|prompt|next|readiness|record-refinement|record-signature|record-role-model-profiles|record-prepare|create-task-packet|task-board-set|wp-traceability-set|prepare-and-packet> [WP-{ID}] [...]`
- current preparation mechanics still live under shared or orchestrator-owned commands
- manual workflow keeps pre-launch work under the Orchestrator; Activation Manager is the governed pre-launch lane for orchestrator-managed workflow, not a second manual authority path
- the Orchestrator remains the live launch and final status authority
- activation-manager now dispatches its mutation actions through the same live Orchestrator implementation so bounded manual activation repair/reference work can happen without exposing a second named command family

## Transitional Shared / Inherited Surfaces

- `just begin-refinement WP-{ID} "<intent>"`
- `just generate-refinement-rubric`
- `just orchestrator-prepare-and-packet WP-{ID}`
- `just mt-populate WP-{ID}`
- `just phase-check STARTUP WP-{ID} CODER`

## Delegated Action Surface

- `just activation-manager record-refinement WP-{ID}` -> delegates to `record-refinement`
- `just activation-manager record-signature WP-{ID} ...` -> delegates to `record-signature`
- `just activation-manager record-role-model-profiles WP-{ID} ...` -> delegates to `record-role-model-profiles`
- `just activation-manager record-prepare WP-{ID} ...` -> delegates to `record-prepare`
- `just activation-manager create-task-packet WP-{ID} "<context>"` -> delegates to `create-task-packet`
- `just activation-manager task-board-set WP-{ID} <STATUS> [reason]` -> delegates to `task-board-set`
- `just activation-manager wp-traceability-set <BASE_WP_ID> <ACTIVE_PACKET_WP_ID> "<context>"` -> delegates to `wp-traceability-set`
- `just activation-manager prepare-and-packet WP-{ID}` -> delegates to `orchestrator-prepare-and-packet`
- this keeps one implementation path while still giving Activation Manager one role-local operator-facing surface

## Role Layout

- `runtime/`
  - role-local runtime notes and future tracked machine state only
- `scripts/`
  - activation-manager-owned entrypoints
- `scripts/lib/`
  - activation-manager-only helper libraries
- `checks/`
  - activation-manager-owned enforcement and readiness checks
- `tests/`
  - governance tests for activation-manager scripts/checks
- `fixtures/`
  - activation-manager-local test data and golden inputs

## Manual Launch Flow

- Generic startup:
  - `just activation-manager startup`
- Prompt brief for a specific WP:
  - `just activation-manager prompt WP-{ID}`
- Current activation state for a WP:
  - `just activation-manager next WP-{ID}`
- Write/read the readiness artifact:
  - `just activation-manager readiness WP-{ID} --write`

## Activation Actions

- `just activation-manager record-refinement WP-{ID}`
- `just activation-manager record-signature WP-{ID} ...`
- `just activation-manager record-role-model-profiles WP-{ID} ...`
- `just activation-manager record-prepare WP-{ID} ...`
- `just activation-manager create-task-packet WP-{ID} "<context>"`
- `just activation-manager task-board-set WP-{ID} <STATUS> [reason]`
- `just activation-manager wp-traceability-set <BASE_WP_ID> <ACTIVE_PACKET_WP_ID> "<context>"`
- `just activation-manager prepare-and-packet WP-{ID}`

The readiness artifact is written to the external governance runtime root under:
- `../gov_runtime/roles/activation_manager/runtime/activation_readiness/WP-{ID}.md`

## Standard Outputs

- signed or blocked `refinement.md`
- optional spec-enrichment patch set plus pointer updates
- normalized signature record / audit entry
- hydrated work packet
- populated microtask scaffolding when the packet declares it
- prepared branch/worktree assignment
- `ACTIVATION_READINESS` handoff for the Orchestrator
