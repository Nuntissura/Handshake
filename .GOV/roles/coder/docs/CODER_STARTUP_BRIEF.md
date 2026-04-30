# Coder Startup Brief

## Status

- SCHEMA_VERSION: `hsk.startup_brief@1`
- STATUS: ACTIVE
- OWNER: MEMORY_MANAGER
- ROLE: CODER

## Use

Use this brief after `just coder-startup`. It is operational memory for implementation lanes.

## Action Cards

### RAM-CODER-WORKTREE-001

- ACTION: WORKTREE_CONFINEMENT
- TRIGGER: before reading or editing files for a WP
- FAILURE_PATTERN: treating the Operator worktree, gov kernel, or `handshake_main` as a coder worktree
- DO: work only in the assigned WP worktree and branch from the packet/session assignment
- DO_NOT: edit `.GOV` through the junction except for coder-owned packet/MT status/evidence fields; never commit `.GOV` from the feature branch
- VERIFY: `just role-startup-topology-check` and `just phase-check STARTUP WP-{ID} CODER <session>` pass
- SOURCE: CODER_PROTOCOL

### RAM-CODER-HANDOFF-001

- ACTION: HANDOFF
- TRIGGER: before claiming implementation complete
- FAILURE_PATTERN: reporting tests or summaries without phase-check handoff proof and spec evidence
- DO: run the packet TEST_PLAN, document evidence, drain required direct-review obligations, then run `just phase-check HANDOFF WP-{ID} CODER`
- DO_NOT: claim done while overlap review items or handoff gate blockers remain
- VERIFY: handoff phase check passes and `## STATUS_HANDOFF` contains required proof
- SOURCE: CODER_PROTOCOL
