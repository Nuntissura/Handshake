# Governance Script Rationalization Matrix (Full)

## METADATA
- AUDIT_ID: AUDIT-20260408-GOVERNANCE-SCRIPT-RATIONALIZATION-MATRIX-FULL
- DATE_UTC: 2026-04-08T10:57:35.716Z
- AUDITOR: Codex ORCHESTRATOR
- SPEC_CURRENT_POINTER: .GOV/spec/SPEC_CURRENT.md
- SPEC_TARGET_RESOLVED: .GOV/spec/Handshake_Master_Spec_v02.180.md
- CODE_TARGET:
  - worktree: `D:\Projects\LLM projects\Handshake\Handshake Worktrees\wt-gov-kernel`
  - branch: `gov_kernel`
  - commit_sha: `168c883cc0edfa4dc72f7424409473708017ea39`
- SCOPE_SUMMARY: Exhaustive one-file-per-row matrix for governance script files under `.GOV`. This is a rationalization aid, not a deletion order.
- OUT_OF_SCOPE: stubs, work packets, task packets, and product code.

## INTERPRETATION
- The rows below cover all governance files under `.GOV` with extensions `.mjs`, `.js`, `.ps1`, or `.sh`.
- `Proposed Action` is a first-pass rationalization label. P0-P3 rows include manual overrides where the cross-check clearly showed blast radius or drift. Most P4 rows are heuristic and should be reviewed in families rather than acted on individually.
- `Surface` means where the file appears to live today: `JUST`, `DOC`, `INTERNAL`, `TEST`, or combinations.
- `CurrentDocs` excludes historical audits, packet bodies, refinements, operator message history, and local legacy notes to reduce archaeology noise.
- For this cleanup track, repo-retired governance scripts/tests are expected to move to the external archive root `../../scripts_archive/` for safekeeping and posterity instead of being hard-deleted.
- Operator review update (2026-04-08): the Family A/B candidate set is currently treated as live or recovery-adjacent surface and is deferred from merge/archive work in the current wave.

## SUMMARY
- Total files in matrix: `273`
- Kind counts:
  - check: `73`
  - entry: `99`
  - lib: `36`
  - other: `2`
  - test: `63`
- Action counts:
  - DEFER_LIVE_SURFACE: `4`
  - DEFER_RECOVERY_SURFACE: `3`
  - KEEP: `107`
  - KEEP_ADD_TEST: `45`
  - KEEP_CORE_SPLIT_TEST_FIRST: `15`
  - KEEP_LIB_REVIEW_WITH_PARENT: `36`
  - KEEP_TEST: `63`

## MATRIX

| Priority | Kind | File | Lines | Surface | Just | CurrentDocs | Inbound | TestRefs | Proposed Action | Basis |
|---|---|---|---:|---|---:|---:|---:|---:|---|---|
| P0 | check | `.GOV/roles_shared/checks/gov-check.mjs` | 97 | JUST+DOC | 1 | 22 | 9 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | curated: central governance gate aggregator; metrics: just=1, docs=22, inbound=9, tests=0 |
| P0 | check | `.GOV/roles_shared/checks/refinement-check.mjs` | 2718 | DOC+INTERNAL | 0 | 4 | 6 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | curated: shared validator core; extremely large; zero direct tests; metrics: just=0, docs=4, inbound=6, tests=0 |
| P0 | entry | `.GOV/roles/orchestrator/scripts/create-task-packet.mjs` | 1508 | JUST+DOC | 1 | 14 | 6 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | curated: activation hub; high blast radius; zero direct tests; metrics: just=1, docs=14, inbound=6, tests=0 |
| P0 | entry | `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs` | 2241 | DOC+INTERNAL | 0 | 2 | 2 | 1 | `KEEP_CORE_SPLIT_TEST_FIRST` | curated: canonical viewport implementation behind shim; metrics: just=0, docs=2, inbound=2, tests=1 |
| P0 | entry | `.GOV/roles/orchestrator/scripts/session-control-command.mjs` | 549 | JUST+DOC | 1 | 5 | 8 | 1 | `KEEP_CORE_SPLIT_TEST_FIRST` | curated: session command multiplexer for start/send/close/cancel; metrics: just=1, docs=5, inbound=8, tests=1 |
| P1 | check | `.GOV/roles_shared/checks/session-control-runtime-check.mjs` | 507 | JUST+DOC | 1 | 3 | 2 | 1 | `KEEP_ADD_TEST` | curated: runtime gate; important but thinly tested; metrics: just=1, docs=3, inbound=2, tests=1 |
| P1 | entry | `.GOV/roles/orchestrator/scripts/manual-relay-dispatch.mjs` | 253 | JUST+DOC | 1 | 4 | 4 | 2 | `KEEP_ADD_TEST` | curated: manual relay dispatch is live and route-sensitive; metrics: just=1, docs=4, inbound=4, tests=2 |
| P1 | entry | `.GOV/roles/orchestrator/scripts/role-session-worktree-add.mjs` | 115 | JUST | 1 | 0 | 3 | 0 | `KEEP_ADD_TEST` | curated: live helper with role-specific branching and no direct tests; metrics: just=1, docs=0, inbound=3, tests=0 |
| P1 | entry | `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs` | 580 | JUST+DOC | 1 | 6 | 3 | 1 | `KEEP_CORE_SPLIT_TEST_FIRST` | curated: role launch surface; high coupling; metrics: just=1, docs=6, inbound=3, tests=1 |
| P1 | entry | `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs` | 860 | JUST+DOC | 1 | 8 | 11 | 2 | `KEEP_CORE_SPLIT_TEST_FIRST` | curated: orchestrator resume/control path; metrics: just=1, docs=8, inbound=11, tests=2 |
| P2 | entry | `.GOV/roles/orchestrator/scripts/manual-relay-next.mjs` | 123 | JUST+DOC | 1 | 3 | 4 | 2 | `KEEP_ADD_TEST` | curated: live manual-lane routing surface; metrics: just=1, docs=3, inbound=4, tests=2 |
| P2 | entry | `.GOV/roles/orchestrator/scripts/session-control-broker.mjs` | 45 | JUST+DOC | 1 | 1 | 1 | 0 | `KEEP_ADD_TEST` | curated: small admin surface with no direct tests; metrics: just=1, docs=1, inbound=1, tests=0 |
| P2 | entry | `.GOV/operator/scripts/operator-viewport-tui.mjs` | 9 | JUST+DOC | 1 | 3 | 2 | 2 | `DEFER_LIVE_SURFACE` | curated: active operator viewport entrypoint via live just recipes and current docs/tests; metrics: just=1, docs=3, inbound=2, tests=2 |
| P2 | entry | `.GOV/roles/orchestrator/scripts/session-control-cancel.mjs` | 27 | JUST+DOC | 1 | 1 | 1 | 0 | `DEFER_LIVE_SURFACE` | curated: live session-cancel entrypoint and protocol-aligned surface; metrics: just=1, docs=1, inbound=1, tests=0 |
| P2 | check | `.GOV/roles_shared/checks/build-order-check.mjs` | 23 | DOC+INTERNAL | 0 | 2 | 1 | 0 | `DEFER_LIVE_SURFACE` | curated: active gov-check chain member and BUILD_ORDER-visible check; metrics: just=0, docs=2, inbound=1, tests=0 |
| P3 | entry | `.GOV/roles/orchestrator/scripts/gov-check-feedback.mjs` | 101 | NONE | 0 | 0 | 0 | 0 | `DEFER_RECOVERY_SURFACE` | curated: recovery/admin-adjacent gov-check failure router; hold until dedicated review; metrics: just=0, docs=0, inbound=0, tests=0 |
| P3 | entry | `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs` | 127 | DOC | 0 | 3 | 0 | 0 | `DEFER_RECOVERY_SURFACE` | curated: documented stub helper retained pending command-surface review; metrics: just=0, docs=3, inbound=0, tests=0 |
| P3 | entry | `.GOV/roles/orchestrator/scripts/session-reset-batch-launch-mode.mjs` | 40 | DOC | 0 | 2 | 0 | 0 | `DEFER_RECOVERY_SURFACE` | curated: documented batch-reset helper retained as recovery/admin surface; metrics: just=0, docs=2, inbound=0, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/active-lane-brief.mjs` | 28 | JUST+DOC | 1 | 7 | 9 | 4 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=7, inbound=9, tests=4 |
| P4 | check | `.GOV/roles_shared/checks/atelier_role_registry_check.mjs` | 280 | DOC+INTERNAL | 0 | 1 | 3 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=3, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/ci-traceability-check.mjs` | 214 | DOC+INTERNAL | 0 | 4 | 1 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=4, inbound=1, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/computed-policy-gate-check.mjs` | 75 | JUST+DOC | 1 | 2 | 3 | 1 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=2, inbound=3, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/cor701-sha.mjs` | 115 | JUST+DOC | 1 | 6 | 1 | 0 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=6, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/deprecation-sunset-check.mjs` | 110 | DOC+INTERNAL | 0 | 3 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=3, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/gate-check.mjs` | 119 | JUST+DOC | 1 | 14 | 6 | 2 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=14, inbound=6, tests=2 |
| P4 | check | `.GOV/roles_shared/checks/governance-reference.mjs` | 107 | DOC+INTERNAL | 0 | 1 | 4 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=4, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/governance-structure-rules.mjs` | 86 | INTERNAL | 0 | 0 | 1 | 0 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=0, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/historical-smoketest-lineage-check.mjs` | 31 | DOC+INTERNAL | 0 | 3 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=3, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/lifecycle-ux-check.mjs` | 93 | DOC+INTERNAL | 0 | 2 | 3 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=2, inbound=3, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/memory-health-check.mjs` | 69 | INTERNAL | 0 | 0 | 1 | 0 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=0, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/merge-progression-truth-check.mjs` | 28 | DOC+INTERNAL | 0 | 1 | 1 | 0 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/oss-register-check.mjs` | 223 | DOC+INTERNAL | 0 | 1 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/packet-closure-monitor-check.mjs` | 82 | DOC+INTERNAL | 0 | 1 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/packet-truth-check.mjs` | 257 | DOC+INTERNAL | 0 | 2 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=2, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/phase1-add-coverage-check.mjs` | 205 | INTERNAL | 0 | 0 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=0, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/protocol-alignment-check.mjs` | 509 | DOC+INTERNAL | 0 | 3 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=3, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/role-worktree-surface-check.mjs` | 42 | INTERNAL | 0 | 0 | 1 | 0 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=0, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/runtime-placement-check.mjs` | 114 | DOC+INTERNAL | 0 | 2 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=2, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/semantic-proof-check.mjs` | 58 | DOC+INTERNAL | 0 | 1 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/session-launch-runtime-check.mjs` | 51 | DOC+INTERNAL | 0 | 1 | 1 | 0 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/session-policy-check.mjs` | 340 | DOC+INTERNAL | 0 | 3 | 3 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=3, inbound=3, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/skeleton-approved.mjs` | 210 | JUST+DOC | 1 | 5 | 4 | 1 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=5, inbound=4, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/spec-debt-registry-check.mjs` | 16 | DOC+INTERNAL | 0 | 1 | 1 | 0 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/spec-eof-appendices-check.mjs` | 261 | JUST+DOC | 1 | 5 | 3 | 2 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=5, inbound=3, tests=2 |
| P4 | check | `.GOV/roles_shared/checks/spec-governance-reference-check.mjs` | 88 | DOC+INTERNAL | 0 | 1 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=2, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/spec-growth-discipline-check.mjs` | 114 | DOC+INTERNAL | 0 | 1 | 1 | 0 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/task-board-check.mjs` | 109 | DOC+INTERNAL | 0 | 6 | 3 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=6, inbound=3, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/task-packet-claim-check.mjs` | 245 | DOC+INTERNAL | 0 | 9 | 3 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=9, inbound=3, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/topology-registry-check.mjs` | 41 | DOC+INTERNAL | 0 | 2 | 1 | 0 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=2, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/workflow-start-readiness-check.mjs` | 26 | DOC | 0 | 1 | 0 | 0 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/worktree-concurrency-check.mjs` | 161 | DOC+INTERNAL | 0 | 6 | 3 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=6, inbound=3, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/wp-activation-traceability-check.mjs` | 136 | DOC+INTERNAL | 0 | 2 | 3 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=2, inbound=3, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/wp-communication-health-check.mjs` | 127 | JUST+DOC | 1 | 9 | 8 | 2 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=9, inbound=8, tests=2 |
| P4 | check | `.GOV/roles_shared/checks/wp-declared-topology-check.mjs` | 43 | JUST+DOC | 1 | 2 | 0 | 0 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=2, inbound=0, tests=0 |
| P4 | check | `.GOV/roles/coder/checks/coder-bootstrap-claim.mjs` | 62 | DOC+INTERNAL | 0 | 1 | 3 | 1 | `KEEP` | heuristic: active check surface; metrics: just=0, docs=1, inbound=3, tests=1 |
| P4 | check | `.GOV/roles/coder/checks/coder-skeleton-checkpoint.mjs` | 99 | JUST+DOC | 1 | 6 | 6 | 2 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=6, inbound=6, tests=2 |
| P4 | check | `.GOV/roles/coder/checks/post-work.mjs` | 156 | JUST+DOC | 1 | 34 | 14 | 3 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=34, inbound=14, tests=3 |
| P4 | check | `.GOV/roles/coder/checks/pre-work.mjs` | 206 | JUST+DOC | 1 | 24 | 13 | 2 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=24, inbound=13, tests=2 |
| P4 | check | `.GOV/roles/orchestrator/checks/orchestrator-startup-truth-check.mjs` | 31 | JUST+DOC | 1 | 1 | 0 | 0 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=1, inbound=0, tests=0 |
| P4 | check | `.GOV/roles/validator/checks/external-validator-brief.mjs` | 349 | JUST+DOC | 1 | 7 | 3 | 2 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=7, inbound=3, tests=2 |
| P4 | check | `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs` | 123 | JUST+DOC | 1 | 8 | 10 | 5 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=8, inbound=10, tests=5 |
| P4 | check | `.GOV/roles/validator/checks/integration-validator-context-brief.mjs` | 67 | JUST+DOC | 1 | 5 | 12 | 6 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=5, inbound=12, tests=6 |
| P4 | check | `.GOV/roles/validator/checks/validator_gates.mjs` | 785 | JUST+DOC | 1 | 13 | 8 | 3 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=13, inbound=8, tests=3 |
| P4 | check | `.GOV/roles/validator/checks/validator-coverage-gaps.mjs` | 66 | JUST+DOC | 1 | 5 | 1 | 1 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=5, inbound=1, tests=1 |
| P4 | check | `.GOV/roles/validator/checks/validator-dal-audit.mjs` | 124 | JUST+DOC | 1 | 10 | 1 | 1 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=10, inbound=1, tests=1 |
| P4 | check | `.GOV/roles/validator/checks/validator-error-codes.mjs` | 221 | JUST+DOC | 1 | 6 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=6, inbound=2, tests=1 |
| P4 | check | `.GOV/roles/validator/checks/validator-git-hygiene.mjs` | 84 | JUST+DOC | 1 | 9 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=9, inbound=2, tests=1 |
| P4 | check | `.GOV/roles/validator/checks/validator-governance-snapshot.mjs` | 234 | JUST+DOC | 1 | 4 | 1 | 1 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=4, inbound=1, tests=1 |
| P4 | check | `.GOV/roles/validator/checks/validator-handoff-check.mjs` | 427 | JUST+DOC | 1 | 9 | 7 | 3 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=9, inbound=7, tests=3 |
| P4 | check | `.GOV/roles/validator/checks/validator-packet-complete.mjs` | 755 | JUST+DOC | 1 | 11 | 8 | 2 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=11, inbound=8, tests=2 |
| P4 | check | `.GOV/roles/validator/checks/validator-phase-gate.mjs` | 67 | JUST+DOC | 1 | 5 | 1 | 1 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=5, inbound=1, tests=1 |
| P4 | check | `.GOV/roles/validator/checks/validator-report-structure-check.mjs` | 563 | JUST+DOC | 1 | 5 | 5 | 3 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=5, inbound=5, tests=3 |
| P4 | check | `.GOV/roles/validator/checks/validator-scan.mjs` | 161 | JUST+DOC | 1 | 11 | 3 | 2 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=11, inbound=3, tests=2 |
| P4 | check | `.GOV/roles/validator/checks/validator-spec-regression.mjs` | 61 | JUST+DOC | 1 | 8 | 0 | 0 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=8, inbound=0, tests=0 |
| P4 | check | `.GOV/roles/validator/checks/validator-traceability.mjs` | 56 | JUST+DOC | 1 | 6 | 2 | 1 | `KEEP` | heuristic: active check surface; metrics: just=1, docs=6, inbound=2, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/debt/spec-debt-close.mjs` | 54 | DOC | 0 | 2 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=2, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/debt/spec-debt-open.mjs` | 108 | JUST+DOC | 1 | 4 | 1 | 1 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=4, inbound=1, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/debt/spec-debt-sync.mjs` | 52 | JUST+DOC | 1 | 5 | 1 | 1 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=5, inbound=1, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/dev/codex-check-test.mjs` | 41 | DOC | 0 | 2 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=2, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/dev/new-api-endpoint.mjs` | 134 | DOC+INTERNAL | 0 | 5 | 1 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=5, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/dev/new-react-component.mjs` | 75 | DOC+INTERNAL | 0 | 5 | 1 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=5, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/dev/scaffold-check.mjs` | 84 | DOC | 0 | 4 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=4, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/generate-refinement-rubric.mjs` | 101 | JUST+DOC | 1 | 2 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=2, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/hooks/install-mt-hook.mjs` | 59 | JUST+DOC | 1 | 2 | 2 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=2, inbound=2, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/hooks/install-validator-guard.mjs` | 75 | JUST+DOC | 1 | 1 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=1, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/hooks/validator-write-guard.mjs` | 44 | INTERNAL | 0 | 0 | 1 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=0, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/memory/memory-capture-from-check.mjs` | 44 | DOC+INTERNAL | 0 | 1 | 5 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=1, inbound=5, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/memory/memory-refresh.mjs` | 119 | JUST+DOC | 1 | 6 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=6, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/protocol-ack.mjs` | 41 | JUST+DOC | 1 | 2 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=2, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/acp-build-id.mjs` | 38 | INTERNAL | 0 | 0 | 1 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=0, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/reclaim-owned-terminals.mjs` | 69 | JUST+DOC | 1 | 1 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=1, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/send-mt-prompt.mjs` | 116 | JUST | 1 | 0 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=0, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/session-control-lib.mjs` | 1434 | DOC+INTERNAL | 0 | 6 | 15 | 2 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=6, inbound=15, tests=2 |
| P4 | entry | `.GOV/roles_shared/scripts/session/session-governance-state-lib.mjs` | 90 | DOC+INTERNAL | 0 | 1 | 7 | 1 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=1, inbound=7, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/session/session-policy.mjs` | 690 | DOC+INTERNAL | 0 | 7 | 57 | 5 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=7, inbound=57, tests=5 |
| P4 | entry | `.GOV/roles_shared/scripts/session/session-registry-lib.mjs` | 1073 | DOC+INTERNAL | 0 | 3 | 39 | 3 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=3, inbound=39, tests=3 |
| P4 | entry | `.GOV/roles_shared/scripts/session/wp-timeline-report.mjs` | 146 | JUST+DOC | 1 | 1 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=1, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/wp-token-budget-lib.mjs` | 118 | INTERNAL | 0 | 0 | 7 | 1 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=0, inbound=7, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/session/wp-token-usage-report.mjs` | 116 | JUST+DOC | 1 | 1 | 1 | 1 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=1, inbound=1, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/session/wp-token-usage-settle.mjs` | 43 | JUST+DOC | 1 | 1 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=1, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/spec-current-check.mjs` | 74 | DOC+INTERNAL | 0 | 3 | 3 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=3, inbound=3, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/artifact-cleanup.mjs` | 51 | JUST+DOC | 1 | 5 | 1 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=5, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/artifact-hygiene-check.mjs` | 35 | JUST+DOC | 1 | 6 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=6, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/backup-push.mjs` | 86 | JUST+DOC | 1 | 3 | 3 | 1 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=3, inbound=3, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/backup-status.mjs` | 70 | JUST+DOC | 1 | 8 | 1 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=8, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/close-wp-branch.mjs` | 144 | JUST+DOC | 1 | 4 | 1 | 1 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=4, inbound=1, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/ensure-permanent-backup-branches.mjs` | 53 | DOC+INTERNAL | 0 | 3 | 1 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=3, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/enumerate-cleanup-targets.mjs` | 85 | JUST+DOC | 1 | 6 | 1 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=6, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/reseed-permanent-worktree-from-main.mjs` | 306 | JUST+DOC | 1 | 9 | 6 | 2 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=9, inbound=6, tests=2 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/sync-all-role-worktrees.mjs` | 57 | JUST+DOC | 1 | 7 | 2 | 1 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=7, inbound=2, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/topology-registry-sync.mjs` | 16 | DOC+INTERNAL | 0 | 3 | 1 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=0, docs=3, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs` | 620 | JUST+DOC | 1 | 3 | 6 | 2 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=3, inbound=6, tests=2 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/failure-memory.mjs` | 79 | JUST+DOC | 1 | 2 | 2 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=2, inbound=2, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/mt-board.mjs` | 84 | JUST+DOC | 1 | 2 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=2, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/wp-closeout-format.mjs` | 89 | JUST+DOC | 1 | 3 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=3, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/wp-heartbeat.mjs` | 258 | JUST+DOC | 1 | 5 | 2 | 2 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=5, inbound=2, tests=2 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/wp-invalidity-flag.mjs` | 39 | JUST+DOC | 1 | 2 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=2, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/wp-operator-rule-restatement.mjs` | 39 | JUST+DOC | 1 | 5 | 0 | 0 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=5, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs` | 1253 | JUST+DOC | 1 | 8 | 10 | 2 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=8, inbound=10, tests=2 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/wp-review-exchange.mjs` | 249 | JUST+DOC | 1 | 6 | 7 | 3 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=6, inbound=7, tests=3 |
| P4 | entry | `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs` | 210 | JUST+DOC | 1 | 6 | 8 | 3 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=6, inbound=8, tests=3 |
| P4 | entry | `.GOV/roles/orchestrator/scripts/task-board-set.mjs` | 256 | JUST+DOC | 1 | 6 | 10 | 2 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=6, inbound=10, tests=2 |
| P4 | entry | `.GOV/roles/orchestrator/scripts/wp-traceability-set.mjs` | 142 | JUST+DOC | 1 | 4 | 3 | 1 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=4, inbound=3, tests=1 |
| P4 | entry | `.GOV/roles/validator/scripts/validator-next.mjs` | 624 | JUST+DOC | 1 | 8 | 9 | 3 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=8, inbound=9, tests=3 |
| P4 | other | `.GOV/tools/handshake-acp-bridge/agent.mjs` | 1034 | JUST+DOC | 1 | 34 | 22 | 6 | `KEEP` | heuristic: active entrypoint; metrics: just=1, docs=34, inbound=22, tests=6 |
| P4 | check | `.GOV/roles_shared/checks/codex-check.mjs` | 168 | DOC+INTERNAL | 0 | 6 | 1 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large check with no direct tests; metrics: just=0, docs=6, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/drive-agnostic-check.mjs` | 181 | DOC+INTERNAL | 0 | 2 | 1 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large check with no direct tests; metrics: just=0, docs=2, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/governance-structure-check.mjs` | 131 | DOC | 0 | 1 | 0 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large check with no direct tests; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/migration-path-truth-check.mjs` | 181 | DOC+INTERNAL | 0 | 4 | 1 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large check with no direct tests; metrics: just=0, docs=4, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/prevention-ladder-check.mjs` | 220 | DOC+INTERNAL | 0 | 1 | 1 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large check with no direct tests; metrics: just=0, docs=1, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/checks/wp-communications-check.mjs` | 174 | DOC+INTERNAL | 0 | 2 | 1 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large check with no direct tests; metrics: just=0, docs=2, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs` | 580 | JUST+DOC | 1 | 1 | 1 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=1, inbound=1, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/build-order-sync.mjs` | 563 | JUST+DOC | 1 | 5 | 7 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=5, inbound=7, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/governance-snapshot.mjs` | 400 | JUST+DOC | 1 | 5 | 2 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=5, inbound=2, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/hooks/post-commit-mt-review-request.mjs` | 186 | INTERNAL | 0 | 0 | 1 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=0, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/memory/governance-memory-cli.mjs` | 294 | JUST+DOC | 1 | 2 | 0 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=2, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/memory/memory-compact.mjs` | 253 | JUST+DOC | 1 | 5 | 1 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=5, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/memory/memory-extract-from-receipts.mjs` | 201 | JUST+DOC | 1 | 2 | 2 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=2, inbound=2, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/memory/memory-extract-from-smoketests.mjs` | 173 | JUST+DOC | 1 | 2 | 2 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=2, inbound=2, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/memory/memory-snapshot.mjs` | 230 | DOC+INTERNAL | 0 | 2 | 7 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=2, inbound=7, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/memory/repomem.mjs` | 411 | JUST+DOC | 1 | 5 | 3 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=5, inbound=3, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs` | 344 | DOC+INTERNAL | 0 | 1 | 1 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=1, inbound=1, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/handshake-acp-client.mjs` | 449 | INTERNAL | 0 | 0 | 3 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=0, inbound=3, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/scan-orphan-terminals.mjs` | 190 | JUST | 1 | 0 | 0 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=0, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/session-control-self-settle-lib.mjs` | 260 | DOC+INTERNAL | 0 | 2 | 5 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=2, inbound=5, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/session/session-stall-scan.mjs` | 173 | JUST+DOC | 1 | 1 | 0 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=1, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/terminal-ownership-lib.mjs` | 301 | DOC+INTERNAL | 0 | 1 | 6 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=1, inbound=6, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/session/wp-lane-health.mjs` | 174 | JUST+DOC | 1 | 3 | 0 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=3, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs` | 778 | DOC+INTERNAL | 0 | 1 | 2 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=1, inbound=2, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/session/wp-token-usage-lib.mjs` | 703 | DOC+INTERNAL | 0 | 1 | 10 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=1, inbound=10, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/backup-snapshot.mjs` | 213 | JUST+DOC | 1 | 9 | 6 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=9, inbound=6, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/delete-local-worktree.mjs` | 677 | JUST+DOC | 1 | 9 | 5 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=9, inbound=5, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/generate-worktree-cleanup-script.mjs` | 324 | JUST+DOC | 1 | 4 | 1 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=4, inbound=1, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/git-topology-lib.mjs` | 383 | DOC+INTERNAL | 0 | 2 | 15 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=2, inbound=15, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/gov-flush.mjs` | 269 | JUST+DOC | 1 | 1 | 0 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=1, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/retire-standalone-checkout.mjs` | 255 | JUST+DOC | 1 | 1 | 0 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=1, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/sync-gov-to-main.mjs` | 174 | JUST+DOC | 1 | 8 | 3 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=8, inbound=3, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/topology/worktree-add.mjs` | 210 | JUST+DOC | 1 | 8 | 5 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=8, inbound=5, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/wp-check-notifications.mjs` | 335 | JUST | 1 | 0 | 10 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=0, inbound=10, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/wp-notification-append.mjs` | 214 | INTERNAL | 0 | 0 | 5 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=0, docs=0, inbound=5, tests=1 |
| P4 | entry | `.GOV/roles_shared/scripts/wp/wp-thread-append.mjs` | 196 | JUST+DOC | 1 | 7 | 4 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=7, inbound=4, tests=0 |
| P4 | entry | `.GOV/roles/coder/scripts/coder-next.mjs` | 430 | JUST+DOC | 1 | 8 | 6 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=8, inbound=6, tests=1 |
| P4 | entry | `.GOV/roles/memory_manager/scripts/launch-memory-manager.mjs` | 447 | JUST+DOC | 1 | 4 | 0 | 0 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=4, inbound=0, tests=0 |
| P4 | entry | `.GOV/roles/orchestrator/scripts/orchestrator-prepare-and-packet.mjs` | 283 | JUST+DOC | 1 | 7 | 4 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=7, inbound=4, tests=1 |
| P4 | entry | `.GOV/roles/orchestrator/scripts/session-registry-status.mjs` | 308 | JUST+DOC | 1 | 12 | 8 | 1 | `KEEP_ADD_TEST` | heuristic: medium/large entrypoint with weak tests; metrics: just=1, docs=12, inbound=8, tests=1 |
| P4 | check | `.GOV/roles_shared/checks/role_mailbox_export_check.mjs` | 545 | DOC+INTERNAL | 0 | 4 | 1 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | heuristic: large check with no direct tests; metrics: just=0, docs=4, inbound=1, tests=0 |
| P4 | check | `.GOV/roles_shared/scripts/checks/canonise-gov.mjs` | 331 | JUST+DOC | 1 | 5 | 0 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | heuristic: large check with no direct tests; metrics: just=1, docs=5, inbound=0, tests=0 |
| P4 | check | `.GOV/roles/coder/checks/post-work-check.mjs` | 1002 | DOC+INTERNAL | 0 | 8 | 2 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | heuristic: large check with no direct tests; metrics: just=0, docs=8, inbound=2, tests=0 |
| P4 | check | `.GOV/roles/coder/checks/pre-work-check.mjs` | 1438 | DOC+INTERNAL | 0 | 5 | 2 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | heuristic: large check with no direct tests; metrics: just=0, docs=5, inbound=2, tests=0 |
| P4 | check | `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs` | 713 | JUST+DOC | 1 | 5 | 2 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | heuristic: large check with no direct tests; metrics: just=1, docs=5, inbound=2, tests=0 |
| P4 | entry | `.GOV/roles_shared/scripts/memory/governance-memory-lib.mjs` | 990 | DOC+INTERNAL | 0 | 2 | 13 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | heuristic: large entrypoint with no direct tests; metrics: just=0, docs=2, inbound=13, tests=0 |
| P4 | entry | `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs` | 562 | JUST+DOC | 1 | 5 | 2 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | heuristic: large entrypoint with no direct tests; metrics: just=1, docs=5, inbound=2, tests=0 |
| P4 | other | `.GOV/tools/vscode-session-bridge/extension.js` | 566 | DOC+INTERNAL | 0 | 10 | 2 | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | heuristic: large entrypoint with no direct tests; metrics: just=0, docs=10, inbound=2, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/artifact-hygiene-lib.mjs` | 433 | DOC+INTERNAL | 0 | 1 | 5 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=5, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/computed-policy-gate-lib.mjs` | 900 | DOC+INTERNAL | 0 | 2 | 7 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=7, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/data-contract-lib.mjs` | 347 | DOC+INTERNAL | 0 | 2 | 6 | 2 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=6, tests=2 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/gate-output-artifact-lib.mjs` | 51 | INTERNAL | 0 | 0 | 2 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=0, inbound=2, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/governance-transaction-utils.mjs` | 73 | INTERNAL | 0 | 0 | 1 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=0, inbound=1, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/historical-smoketest-lineage-lib.mjs` | 287 | DOC+INTERNAL | 0 | 1 | 2 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=2, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/merge-progression-truth-lib.mjs` | 425 | DOC+INTERNAL | 0 | 1 | 7 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=7, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/packet-closure-monitor-lib.mjs` | 390 | DOC+INTERNAL | 0 | 1 | 7 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=7, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs` | 185 | DOC+INTERNAL | 0 | 2 | 6 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=6, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/refinement-brief-lib.mjs` | 98 | DOC+INTERNAL | 0 | 1 | 2 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=2, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/role-resume-utils.mjs` | 719 | DOC+INTERNAL | 0 | 2 | 23 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=23, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/runtime-paths.mjs` | 460 | DOC+INTERNAL | 0 | 10 | 142 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=10, inbound=142, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/scope-surface-lib.mjs` | 380 | DOC+INTERNAL | 0 | 2 | 17 | 3 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=17, tests=3 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/semantic-proof-lib.mjs` | 160 | DOC+INTERNAL | 0 | 1 | 5 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=5, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs` | 172 | DOC+INTERNAL | 0 | 1 | 6 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=6, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/signed-scope-surface-lib.mjs` | 575 | DOC+INTERNAL | 0 | 2 | 4 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=4, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/spec-debt-packet-lib.mjs` | 132 | DOC+INTERNAL | 0 | 1 | 3 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=3, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/spec-debt-registry-lib.mjs` | 157 | DOC+INTERNAL | 0 | 1 | 5 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=5, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/validator-gate-paths.mjs` | 52 | DOC+INTERNAL | 0 | 1 | 10 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=10, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/validator-report-profile-lib.mjs` | 48 | DOC+INTERNAL | 0 | 1 | 5 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=5, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/wp-authority-projection-lib.mjs` | 223 | DOC+INTERNAL | 0 | 1 | 7 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=7, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/wp-comm-sqlite.mjs` | 137 | INTERNAL | 0 | 0 | 1 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=0, inbound=1, tests=0 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs` | 1333 | DOC+INTERNAL | 0 | 2 | 12 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=12, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs` | 796 | DOC+INTERNAL | 0 | 3 | 33 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=3, inbound=33, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/wp-declared-topology-lib.mjs` | 245 | DOC+INTERNAL | 0 | 1 | 6 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=6, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/wp-microtask-lib.mjs` | 335 | DOC+INTERNAL | 0 | 1 | 4 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=4, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/wp-relay-escalation-lib.mjs` | 229 | INTERNAL | 0 | 0 | 6 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=0, inbound=6, tests=1 |
| P4 | lib | `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs` | 199 | DOC+INTERNAL | 0 | 2 | 3 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=3, tests=1 |
| P4 | lib | `.GOV/roles/coder/scripts/lib/coder-governance-lib.mjs` | 258 | DOC+INTERNAL | 0 | 2 | 4 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=4, tests=1 |
| P4 | lib | `.GOV/roles/orchestrator/scripts/lib/manual-relay-envelope-lib.mjs` | 146 | DOC+INTERNAL | 0 | 1 | 4 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=4, tests=1 |
| P4 | lib | `.GOV/roles/orchestrator/scripts/lib/orchestrator-steer-lib.mjs` | 13 | INTERNAL | 0 | 0 | 4 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=0, inbound=4, tests=1 |
| P4 | lib | `.GOV/roles/validator/scripts/lib/committed-validation-evidence-lib.mjs` | 204 | INTERNAL | 0 | 0 | 6 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=0, inbound=6, tests=1 |
| P4 | lib | `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs` | 562 | DOC+INTERNAL | 0 | 2 | 4 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=4, tests=1 |
| P4 | lib | `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs` | 302 | DOC+INTERNAL | 0 | 2 | 2 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=2, inbound=2, tests=1 |
| P4 | lib | `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs` | 565 | DOC+INTERNAL | 0 | 1 | 9 | 1 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=1, inbound=9, tests=1 |
| P4 | lib | `.GOV/roles/validator/scripts/lib/validator-product-targets-lib.mjs` | 74 | INTERNAL | 0 | 0 | 5 | 0 | `KEEP_LIB_REVIEW_WITH_PARENT` | heuristic: shared helper; rationalize with parent entrypoints; metrics: just=0, docs=0, inbound=5, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/active-lane-brief.test.mjs` | 304 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/artifact-hygiene-lib.test.mjs` | 158 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/computed-policy-gate-lib.test.mjs` | 287 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/cwd-agnostic-packet-checks.test.mjs` | 43 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/cwd-agnostic-shared-checks.test.mjs` | 141 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/data-contract-lib.test.mjs` | 92 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/delete-local-worktree.test.mjs` | 45 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/ensure-wp-communications.test.mjs` | 244 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/generate-post-run-audit-skeleton.test.mjs` | 262 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/historical-smoketest-lineage-lib.test.mjs` | 71 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/justfile-gov-root-quoting.test.mjs` | 26 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/merge-progression-truth-lib.test.mjs` | 156 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/new-packet-law-regression.test.mjs` | 448 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs` | 197 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/protocol-alignment-check.test.mjs` | 22 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/reseed-permanent-worktree-from-main.test.mjs` | 85 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/role-resume-utils.test.mjs` | 253 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/runtime-paths.test.mjs` | 128 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/runtime-placement-check.test.mjs` | 76 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/scope-surface-lib.test.mjs` | 71 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/session-control-lib.test.mjs` | 125 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/session-control-self-settle-lib.test.mjs` | 215 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/session-governance-state-lib.test.mjs` | 140 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/session-registry-lib.test.mjs` | 219 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/signed-scope-compatibility-lib.test.mjs` | 81 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/signed-scope-surface-lib.test.mjs` | 255 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/terminal-ownership-lib.test.mjs` | 168 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-auto-relay-paths.test.mjs` | 29 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-check-notifications.test.mjs` | 126 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs` | 1675 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-communications-lib.test.mjs` | 282 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-declared-topology-lib.test.mjs` | 170 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-heartbeat.test.mjs` | 211 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-microtask-lib.test.mjs` | 174 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-receipt-append.test.mjs` | 1267 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-relay-escalation-lib.test.mjs` | 122 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs` | 163 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-timeline-lib.test.mjs` | 320 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-token-budget-lib.test.mjs` | 77 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles_shared/tests/wp-token-usage-lib.test.mjs` | 240 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/coder/tests/coder-command-surface.test.mjs` | 34 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/coder/tests/coder-doc-command-surface.test.mjs` | 44 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/coder/tests/coder-entrypoint-path-safety.test.mjs` | 23 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/coder/tests/coder-governance-lib.test.mjs` | 175 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/orchestrator/tests/manual-relay-envelope-lib.test.mjs` | 131 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/orchestrator/tests/manual-relay-next.test.mjs` | 148 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/orchestrator/tests/operator-monitor-tui.test.mjs` | 21 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/orchestrator/tests/orchestrator-command-surface.test.mjs` | 55 | TEST | 0 | 2 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=2, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/orchestrator/tests/orchestrator-doc-command-surface.test.mjs` | 44 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs` | 104 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/orchestrator/tests/orchestrator-runtime-safety.test.mjs` | 26 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/orchestrator/tests/orchestrator-steer-lib.test.mjs` | 32 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/orchestrator/tests/session-launch-governance.test.mjs` | 77 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/committed-validation-evidence-lib.test.mjs` | 90 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs` | 554 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs` | 180 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/validator-command-surface.test.mjs` | 76 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/validator-doc-command-surface.test.mjs` | 44 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/validator-entrypoint-path-safety.test.mjs` | 22 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/validator-governance-lib.test.mjs` | 492 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/validator-next.test.mjs` | 74 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/validator-report-structure-check.test.mjs` | 831 | TEST | 0 | 1 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=1, inbound=0, tests=0 |
| P4 | test | `.GOV/roles/validator/tests/validator-scan-context.test.mjs` | 19 | TEST | 0 | 0 | 0 | 0 | `KEEP_TEST` | heuristic: governance test asset; metrics: just=0, docs=0, inbound=0, tests=0 |
| P4 | check | `.GOV/roles/validator/checks/validator-hygiene-full.mjs` | 29 | JUST+DOC | 1 | 7 | 1 | 1 | `DEFER_LIVE_SURFACE` | curated: active validator command referenced across docs/tests/packets; metrics: just=1, docs=7, inbound=1, tests=1 |

## COMMAND LOG
- `rg --files .GOV`
- `just --summary`
- targeted `Get-Content` and `rg -n` cross-checks on `justfile`, orchestrator scripts, shared checks, README surfaces, and active governance docs
- `git branch --show-current`
- `git rev-parse HEAD`

## NOTES
- This matrix is intended to support the next pass: family-by-family cleanup decisions and a narrower implementation WP.
- If acted on mechanically, the safest order is: split/test the P0/P1 files, keep Family A/B stable while they remain live or recovery-adjacent, then look for genuinely cold scripts elsewhere.
- Where a row eventually results in retirement, archive the source file under `../../scripts_archive/`.
