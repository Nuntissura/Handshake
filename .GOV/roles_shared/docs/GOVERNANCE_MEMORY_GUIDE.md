# Governance Memory Guide

Navigation and operating guide for the repo-governance memory system. Command syntax authority remains `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`; lifecycle law remains `.GOV/codex/Handshake_Codex_v1.4.md` and the active role protocol.

## Runtime

- Store: `../gov_runtime/roles_shared/GOVERNANCE_MEMORY.db`
- Memory types: `procedural`, `semantic`, `episodic`
- Conversation checkpoints: `just repomem ...`
- Mid-session durable memory: `just memory-capture ...`
- Startup refresh: role startup and `just gov-check` run `just memory-refresh`

## Startup Injection

- Coder: procedural fail-log memories, 1500-token budget.
- Validator roles: procedural + semantic memories, 1500-token budget.
- Orchestrator: cross-WP `GOVERNANCE MEMORY` envelope up to 15000 tokens, with dedicated recent-failure, hygiene-report, prior-day-decision, scored-pattern, and snapshot slices.

Injected memory is advisory. If memory conflicts with the packet, receipts, runtime status, or code state, the live authority surface wins.

## Required Role Habits

- Open a repomem session before governed mutation: `just repomem open "<purpose>" --role ROLE [--wp WP-{ID}]`.
- Capture tool failures immediately: `just memory-capture procedural "<what failed and the fix>" --role ROLE [--wp WP-{ID}]`.
- Close sessions with both substantive content and `--decisions`.
- Use `just memory-search "<query>"` for full detail when startup injection surfaces only a summary.

## Maintenance

- `just memory-stats`: health overview.
- `just memory-refresh --force-compact`: force extraction and compaction.
- `just memory-patterns`: cross-WP pattern synthesis.
- `just launch-memory-manager-session`: governed Memory Manager review when deterministic maintenance is not enough.
