# Shared Checks Bundle

Repo-shared checks live here.

## Check Result Output

- Migrated runners print compact model-visible lines as `OK|WARN|FAIL | summary`.
- Full stdout/stderr or structured result detail is appended to `gov_runtime/check_details.jsonl` for repo-wide checks.
- WP-scoped checks append to `gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-{ID}/check_details.jsonl`.
- Use `--verbose` on migrated runners to print the structured detail entry for human debugging.

## Artifact Absorbers

- `RGF-244` artifact normalizers live in `../scripts/lib/artifact-normalizers/`.
- Absorbers are additive pre-validation shims only; checks and validators still own rejection.
- Applied absorbers append hit rows to `gov_runtime/absorber_hits.jsonl` for operator review.

## Heuristic-Risk MT Classification

- `heuristic-risk-check.mjs WP-{ID} [--json]` classifies declared MT packets for fuzzy/adversarial implementation risk.
- `HEURISTIC_RISK=YES` MTs require corpus/property/negative evidence and project strategy-escalation fields into microtask contracts.
- Repeated non-PASS review responses on a heuristic-risk MT emit `HEURISTIC_RISK_STRATEGY_ESCALATION` before the generic fix-cycle cap.

## Turn-Boundary Nudges

- `../scripts/session/nudge-queue-lib.mjs` owns the RGF-245 queue primitive.
- Enqueue validates typed payloads, writes FIFO JSON files, enforces a depth cap, and stores files below `gov_runtime/nudges/`.
- Drain uses atomic `.claimed` rename, TTL expiry, stale-claim recovery, and failure requeue.

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
