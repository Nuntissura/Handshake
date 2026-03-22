# Shared Scripts Bundle

Repo-shared scripts live here.

## Top-Level Shared Entry Scripts

- `spec-current-check.mjs`
- `build-order-sync.mjs`
- `governance-snapshot.mjs`
- `protocol-ack.mjs`

## Sub-bundles

- `hooks/`
  - shared git-hook plumbing
- `lib/`
  - shared implementation libraries
- `debt/`
  - governed spec-debt commands
- `audit/`
  - generated audit skeletons from packet/runtime/gate/session evidence
- `session/`
  - session policy, broker, registry, and ACP bridge helpers
- `topology/`
  - backup, worktree, cleanup, and topology helpers
- `wp/`
  - WP communications and heartbeat helpers
- `dev/`
  - shared developer scaffolds and scaffold checks
