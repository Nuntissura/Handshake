# Shared Checks Bundle

Repo-shared checks live here.

## Check Result Output

- Migrated runners print compact model-visible lines as `OK|WARN|FAIL | summary`.
- Full stdout/stderr or structured result detail is appended to `gov_runtime/check_details.jsonl` for repo-wide checks.
- WP-scoped checks append to `gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-{ID}/check_details.jsonl`.
- Use `--verbose` on migrated runners to print the structured detail entry for human debugging.

## Core Governance Gates

- `gov-check.mjs`
- `phase-check.mjs`
- `refinement-check.mjs`
- `task-board-check.mjs`
- `packet-truth-check.mjs`
- `historical-smoketest-lineage-check.mjs`
- `task-packet-claim-check.mjs`
- `build-order-check.mjs`
- `topology-registry-check.mjs`

## Cross-Role Runtime / Session Checks

- `session-policy-check.mjs`
- `session-launch-runtime-check.mjs`
- `session-control-runtime-check.mjs`
- `workflow-start-readiness-check.mjs`
- `worktree-concurrency-check.mjs`
- `wp-communications-check.mjs`
- `wp-activation-traceability-check.mjs`
- `cache-stability-check.mjs`

## Shared Proof / Spec Integrity Checks

- `packet-closure-monitor-check.mjs`
- `semantic-proof-check.mjs`
- `computed-policy-gate-check.mjs`
- `spec-debt-registry-check.mjs`
- `spec-eof-appendices-check.mjs`
- `spec-governance-reference-check.mjs`
- `spec-growth-discipline-check.mjs`
- `deprecation-sunset-check.mjs`

## Shared Repo Checks

- `codex-check.mjs`
- `cor701-sha.mjs`
- `ci-traceability-check.mjs`
- `governance-reference.mjs`
- `oss-register-check.mjs`
- `role_mailbox_export_check.mjs`
- `atelier_role_registry_check.mjs`
- `drive-agnostic-check.mjs`
- `migration-path-truth-check.mjs`
- `prevention-ladder-check.mjs`
- `governance-structure-check.mjs`
- `lifecycle-ux-check.mjs`
