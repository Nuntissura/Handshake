# Activation Manager Bundle

This README is navigational only.
Authoritative folder-placement law for the Activation Manager bundle lives in `.GOV/codex/Handshake_Codex_v1.4.md` plus `ACTIVATION_MANAGER_PROTOCOL.md`.

## Primary Live Docs

- `ACTIVATION_MANAGER_PROTOCOL.md`

## Role Purpose

- bounded pre-launch governance authoring for refinement, spec enrichment, signature recording, packet hydration, microtask preparation, worktree preparation, and activation-readiness review

## Migration Status

- manual workflow keeps pre-launch work under the Orchestrator; Activation Manager is the governed pre-launch lane for orchestrator-managed workflow, not a second manual authority path
- the Orchestrator remains the live launch and final status authority

## Role Layout

- `runtime/`
  - role-local runtime notes and future tracked machine state only

## Manual Launch Flow

- Startup: read the Codex, this protocol and the assigned MT, then continue from the MT JSON status
- Write the readiness artifact by hand

## Activation Actions

- author the packet or refinement JSON from the V2 templates by hand
- edit the task board row by hand
- edit the WP traceability registry row by hand

The readiness artifact is written to the external governance runtime root under:
- `../gov_runtime/roles/activation_manager/runtime/activation_readiness/WP-{ID}.md`

## Standard Outputs

- signed or blocked `refinement.md`
- optional spec-enrichment patch set plus indexed manifest/SPEC_CURRENT updates
- normalized signature record / audit entry
- hydrated work packet
- populated microtask scaffolding when the packet declares it
- prepared branch/worktree assignment
- `ACTIVATION_READINESS` handoff for the Orchestrator
