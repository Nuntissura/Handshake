# Repo Governance Changelog

## Metadata

- SCHEMA_VERSION: `hsk.repo_governance_changelog@0.1`
- STATUS: ACTIVE
- PURPOSE: durable governance-only change history for the repo governance kernel
- VERSIONING_RULE: `CHANGESET_VERSION` uses sortable `YYYY.MM.DD.N`
- LINKAGE_RULE: every entry must cite a stable `CHANGESET_ID` plus the driving `AUDIT_ID` and/or `SMOKETEST_REVIEW_ID`
- RELATED_TASK_BOARD: `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`

## Linkage Keys

- AUDIT_ID: `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
- SMOKETEST_REVIEW_ID: `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
- AUDIT_ID: `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SMOKETEST_REVIEW_ID: `SMOKETEST-REVIEW-20260325-CONTRACT-HARDENING-V1`
- AUDIT_ID: `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SMOKETEST_REVIEW_ID: `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- HISTORICAL_COMPARISON_AUDIT_ID: `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`

## Entries

### 2026.04.27.08 / GOV-CHANGE-20260427-08

- STATUS: APPLIED
- SUMMARY: completed RGF-249 predecessor-session lookup for compaction and restart
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: implement the open harness-pattern items from the implementation brief, including predecessor-session lookup.
  - `RGF-249`
- FOLLOW_ON_ITEMS:
  - `RGF-233`
- FILES_CHANGED:
  - `.GOV/roles_shared/scripts/session/predecessor-lookup-lib.mjs`
  - `.GOV/roles_shared/scripts/session/role-self-prime.mjs`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/tests/predecessor-lookup-lib.test.mjs`
  - `.GOV/roles_shared/tests/role-self-prime.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: Governed sessions now write compact append-only event logs under the external governance runtime `roles_shared/WP_SESSIONS/<WP_ID>/<SESSION_ID>/events.jsonl`. `role-self-prime` injects a bounded same-role predecessor summary when available, omits it for first sessions, and treats the current session as predecessor during `PreCompact`. Receipt emission and ACP session-command/tool telemetry feed the event stream without crossing role boundaries.
- VERIFICATION:
  - `node --check .GOV/roles_shared/scripts/session/predecessor-lookup-lib.mjs`
  - `node --check .GOV/roles_shared/scripts/session/role-self-prime.mjs`
  - `node --check .GOV/tools/handshake-acp-bridge/agent.mjs`
  - `node --check .GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `node --test .GOV/roles_shared/tests/predecessor-lookup-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/role-self-prime.test.mjs`
  - `just gov-check`

### 2026.04.27.07 / GOV-CHANGE-20260427-07

- STATUS: APPLIED
- SUMMARY: completed RGF-247 mechanical-track validator-as-tool-result
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: implement the open harness-pattern items from the implementation brief, including mechanical-track validator helpers.
  - `RGF-247`
- FOLLOW_ON_ITEMS:
  - `RGF-249`
- FILES_CHANGED:
  - `.GOV/roles/wp_validator/scripts/wp-validator-mechanical-track.mjs`
  - `.GOV/roles/wp_validator/tests/wp-validator-mechanical-track.test.mjs`
  - `.GOV/roles_shared/scripts/hooks/post-commit-mt-review-request.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/lib/computed-policy-gate-lib.mjs`
  - `.GOV/roles_shared/checks/computed-policy-gate-check.mjs`
  - `.GOV/roles_shared/schemas/WP_RECEIPT.schema.json`
  - `.GOV/roles_shared/schemas/inter_role_verbs/MT_VERDICT.schema.json`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `justfile`
- OUTCOME: Per-MT mechanical validation can now run inline via `wp-validator-mechanical-track`, producing typed `MT_VERDICT_MECHANICAL` receipts with boundary, scope, file-list, build-evidence, concerns, and helper invocation identity. The post-commit MT hook runs the helper before sending the judgment-track review request; mechanical FAIL routes coder remediation immediately and skips WP Validator ACP launch. Computed closeout policy now requires both mechanical and judgment MT PASS receipts for receipt-aware packet formats.
- VERIFICATION:
  - `node --check .GOV/roles/wp_validator/scripts/wp-validator-mechanical-track.mjs`
  - `node --check .GOV/roles_shared/scripts/hooks/post-commit-mt-review-request.mjs`
  - `node --test .GOV/roles/wp_validator/tests/wp-validator-mechanical-track.test.mjs`
  - `node --test .GOV/roles_shared/tests/inter-role-verb-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/computed-policy-gate-lib.test.mjs`
  - `just gov-check`

### 2026.04.27.06 / GOV-CHANGE-20260427-06

- STATUS: APPLIED
- SUMMARY: queued memory-system follow-on tranche `RGF-251` through `RGF-254` after governance-memory audit on the latest WP dossier
- CHANGE_TYPE: GOVERNANCE_PLANNING
- DRIVER_EVIDENCE:
  - 2026-04-27 Operator directive: file RGFs to fix memory-system gaps surfaced by audit on `DOSSIER_20260425_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md`.
  - Audit findings: Integration Validator (5 entries) and Activation Manager (4 entries) recorded only `OPEN`/`CLOSE` pairs across the WP run; legacy `VALIDATOR_PROTOCOL.md` repomem section lacks `DECISION` and `FAIL CAPTURE` clauses; orchestrator startup injects only ~220 tokens / 8 lines and surfaces procedural failures only when `access_count >= 5`; no recent `MEMORY_HYGIENE_REPORT.md` exists, indicating the ACP-launched memory_manager intelligent review has not been running.
- FOLLOW_ON_ITEMS:
  - `RGF-251` Integration-Validator and Activation-Manager Repomem Density
  - `RGF-252` VALIDATOR Protocol Repomem Parity
  - `RGF-253` Orchestrator-Startup Memory Injection Redesign
  - `RGF-254` Memory Manager Intelligent-Review Cadence
- FILES_CHANGED:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

### 2026.04.27.05 / GOV-CHANGE-20260427-05

- STATUS: APPLIED
- SUMMARY: completed RGF-248 named-verb inter-role message schema
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: implement the open harness-pattern items from the implementation brief, including named-verb inter-role traffic.
  - `RGF-248`
- FOLLOW_ON_ITEMS:
  - `RGF-247`
  - `RGF-249`
- FILES_CHANGED:
  - `.GOV/roles_shared/scripts/lib/inter-role-verb-lib.mjs`
  - `.GOV/roles_shared/schemas/inter_role_verbs/*.schema.json`
  - `.GOV/roles_shared/schemas/WP_RECEIPT.schema.json`
  - `.GOV/roles_shared/schemas/NUDGE_PAYLOAD.schema.json`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/session/nudge-queue-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs`
  - `.GOV/roles_shared/checks/verb-coverage-check.mjs`
  - `.GOV/roles_shared/checks/gov-check.mjs`
  - `.GOV/roles_shared/tests/inter-role-verb-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/tests/nudge-queue-lib.test.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/templates/WP_RECEIPTS_TEMPLATE.jsonl`
  - `justfile`
- OUTCOME: The governance kernel now defines the initial eight inter-role verbs (`MT_HANDOFF`, `MT_VERDICT`, `MT_REMEDIATION_REQUIRED`, `WP_HANDOFF`, `INTEGRATION_VERDICT`, `CONCERN`, `PHASE_TRANSITION`, `RELAUNCH_REQUEST`) with schema files and a shared validator/renderer. `wp-receipt-append` accepts `--verb` and persists validated `verb` / `verb_body` fields while legacy receipts continue to validate. Nudge payloads validate matching verb bodies, validator verdict readers prefer verb verdict fields, timeline/dossier projection renders verb receipts into human-readable lines, and `verb-coverage-check` reports adoption by role pair.
- VERIFICATION:
  - `node --check .GOV/roles_shared/scripts/lib/inter-role-verb-lib.mjs`
  - `node --check .GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `node --check .GOV/roles_shared/scripts/session/nudge-queue-lib.mjs`
  - `node --check .GOV/roles_shared/checks/verb-coverage-check.mjs`
  - `node --test .GOV/roles_shared/tests/inter-role-verb-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `node --test .GOV/roles_shared/tests/nudge-queue-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `node .GOV/roles_shared/checks/verb-coverage-check.mjs`
  - `just docs-check`
  - `just build-order-sync`
  - `git diff --check`
  - `just gov-check`

### 2026.04.27.04 / GOV-CHANGE-20260427-04

- STATUS: APPLIED
- SUMMARY: completed RGF-246 hook-driven session self-rehydration
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: implement the open harness-pattern items from the implementation brief, including hook self-rehydration.
  - `RGF-246`
- FOLLOW_ON_ITEMS:
  - `RGF-248`
  - `RGF-247`
  - `RGF-249`
- FILES_CHANGED:
  - `.GOV/roles_shared/scripts/session/role-self-prime.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/hooks/templates/claude/settings-autonomous.json`
  - `.GOV/hooks/templates/codex/settings-autonomous.json`
  - `.GOV/hooks/templates/cursor/settings-autonomous.json`
  - `.GOV/hooks/templates/gemini/settings-autonomous.json`
  - `.GOV/hooks/templates/ollama-resident/settings-autonomous.json`
  - `.GOV/roles_shared/tests/role-self-prime.test.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `justfile`
- OUTCOME: Governed role startup now defaults to a compact `ROLE_SELF_PRIME_HOOK` stub that directs the spawned role to build its effective context through `role-self-prime` from terminal-record fallback, packet projection, runtime status, MT board, active-lane brief, and bounded memory injection. Launch and session-control paths export hook environment variables, provider templates define `SessionStart` and `PreCompact` self-prime commands, PreCompact can write the fresh prompt prefix into a compaction summary, and `--inline-prompt` preserves the legacy inline repair escape hatch.
- VERIFICATION:
  - `node --check .GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `node --check .GOV/roles_shared/scripts/session/role-self-prime.mjs`
  - `node --check .GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `node --check .GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `node --test .GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/role-self-prime.test.mjs`
  - `node --test .GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `node .GOV/roles_shared/checks/task-board-check.mjs`
  - `just docs-check`
  - `just build-order-sync`
  - `git diff --check`
  - `just gov-check`

### 2026.04.27.03 / GOV-CHANGE-20260427-03

- STATUS: APPLIED
- SUMMARY: completed RGF-245 turn-boundary nudge queue with atomic claim
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: implement the open harness-pattern items from the implementation brief, including turn-boundary nudge delivery.
  - `RGF-245`
- FOLLOW_ON_ITEMS:
  - `RGF-246`
  - `RGF-248`
- FILES_CHANGED:
  - `.GOV/roles_shared/scripts/session/nudge-queue-lib.mjs`
  - `.GOV/roles_shared/scripts/session/nudge-queue.mjs`
  - `.GOV/roles_shared/schemas/NUDGE_PAYLOAD.schema.json`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles_shared/tests/nudge-queue-lib.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/checks/README.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `justfile`
- OUTCOME: Governance nudges now have a durable per-session queue under `gov_runtime/nudges/` with typed payload validation, FIFO JSON filenames, atomic `.claimed` drain, TTL expiry, stale-claim recovery, failure requeue, and queue-depth caps. Governed startup prompt assembly drains queued nudges at a safe boundary, active-lane/operator views expose nudge depth, and non-emergency `orchestrator-steer-next` `SEND_PROMPT` delivery now enqueues `STEER` nudges unless `--now`/`--direct` is explicitly used.
- VERIFICATION:
  - `node --test .GOV/roles_shared/tests/nudge-queue-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `node --test .GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `node --test .GOV/roles/orchestrator/tests/operator-monitor-tui.test.mjs .GOV/roles/orchestrator/tests/orchestrator-steer-lib.test.mjs`
  - `node --check` on modified nudge queue, session, active-lane, orchestrator-steer, and operator-monitor scripts
  - `just nudge-depth CODER:WP-TEST-NUDGE-v1`
  - `node .GOV/roles_shared/checks/task-board-check.mjs`
  - `just docs-check`
  - `just build-order-sync`
  - `git diff --check`
  - `just gov-check`

### 2026.04.27.02 / GOV-CHANGE-20260427-02

- STATUS: APPLIED
- SUMMARY: completed RGF-250 heuristic-risk classification and strategy escalation
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: implement the open harness-pattern items from the implementation brief and add heuristic-risk classification plus strategy escalation for fuzzy MT loops.
  - `RGF-250`
- FOLLOW_ON_ITEMS:
  - `RGF-245`
  - `RGF-246`
- FILES_CHANGED:
  - `.GOV/roles_shared/scripts/lib/heuristic-risk-lib.mjs`
  - `.GOV/roles_shared/checks/heuristic-risk-check.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-microtask-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-review-exchange.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RECEIPT.schema.json`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/mt-board.mjs`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/templates/MICRO_TASK_TEMPLATE.md`
  - `.GOV/roles_shared/tests/heuristic-risk-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-communications-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `justfile`
- OUTCOME: Declared MT files are now mechanically classified for fuzzy/adversarial heuristic risk. Positive classifications project into review microtask contracts with required evidence, strategy-escalation class, and a two-cycle strategy threshold. Receipt append enriches contracts before persistence and emits `HEURISTIC_RISK_STRATEGY_ESCALATION` notifications to Orchestrator when repeated non-PASS review responses hit that threshold, so counterexample loops shift strategy before the generic 3-cycle cap. Coder, WP Validator, and Orchestrator prompts/protocols now surface the rule.
- VERIFICATION:
  - `node --test .GOV/roles_shared/tests/heuristic-risk-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/wp-communications-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/wp-microtask-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `node --test .GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `node --check` on modified heuristic-risk, MT, receipt, review, session, and active-lane scripts
  - `just heuristic-risk-check WP-1-Calendar-Sync-Engine-v3`
  - `node .GOV/roles_shared/checks/task-board-check.mjs`
  - `just docs-check`
  - `just build-order-sync`
  - `git diff --check`
  - `node .GOV/roles_shared/checks/topology-bundle-check.mjs`
  - `node .GOV/roles_shared/checks/packet-truth-check.mjs`
  - `just gov-check`

### 2026.04.27.01 / GOV-CHANGE-20260427-01

- STATUS: APPLIED
- SUMMARY: completed RGF-244 deterministic artifact-malformation absorbers
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: implement the open harness-pattern items from the implementation brief, including deterministic artifact-malformation absorption before repair loops.
  - `RGF-244`
- FOLLOW_ON_ITEMS:
  - `RGF-250`
  - `RGF-245`
- FILES_CHANGED:
  - `.GOV/roles_shared/scripts/lib/artifact-normalizers/`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RECEIPT.schema.json`
  - `.GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `.GOV/roles_shared/checks/packet-truth-check.mjs`
  - `.GOV/roles/orchestrator/scripts/closeout-repair.mjs`
  - `.GOV/roles_shared/tests/artifact-normalizers/`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/checks/README.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: Shared artifact absorbers now cover line endings, trailing newlines, smart quotes, unicode dashes, JSON-string arrays, bullet/heading-prefixed fields, key-value whitespace, Windows path escapes, nullish field values, and unambiguous heading-level drift. Receipt append normalizes before persist and records applied absorbers in receipt metadata. Validator-report and packet-truth checks normalize before evaluation, and closeout repair runs an absorber pre-pass before entering mechanical repair classification. Applied absorbers append hit rows to `gov_runtime/absorber_hits.jsonl`.
- VERIFICATION:
  - `node --test .GOV/roles_shared/tests/artifact-normalizers/normalizers.test.mjs`
  - `node --test .GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `node .GOV/roles_shared/checks/packet-truth-check.mjs`
  - `node .GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `node --test .GOV/roles/orchestrator/tests/closeout-repair.test.mjs`
  - `node .GOV/roles_shared/checks/topology-bundle-check.mjs`
  - `git diff --check`
  - `just gov-check`

### 2026.04.26.09 / GOV-CHANGE-20260426-09

- STATUS: APPLIED
- SUMMARY: completed RGF-243 compact check output and durable detail logging
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: implement the open harness-pattern items from the implementation brief, starting with tool-result audit/model asymmetry.
  - `RGF-243`
- FOLLOW_ON_ITEMS:
  - `RGF-244`
  - `RGF-250`
- FILES_CHANGED:
  - `.GOV/roles_shared/scripts/lib/check-result-lib.mjs`
  - `.GOV/roles_shared/checks/gov-check.mjs`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/checks/wp-communication-health-check.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles_shared/tests/check-result-lib.test.mjs`
  - `.GOV/roles_shared/tests/check-details-log.test.mjs`
  - `.GOV/roles_shared/checks/README.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: `gov-check` now runs checks as subprocess steps, emits compact `OK|FAIL | summary` lines, records stdout/stderr into repo-scope `check_details.jsonl`, and continues collecting step failures. `phase-check` records per-step and final WP-scoped detail rows while preserving existing status markers and artifacts. Workflow Dossier sync and the operator monitor `CHECKS` view read the WP-scoped detail log for human diagnostics.
- VERIFICATION:
  - `node --test .GOV/roles_shared/tests/check-result-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/check-details-log.test.mjs`
  - `node --test .GOV/roles_shared/tests/phase-check.test.mjs`
  - `node --check .GOV/roles_shared/checks/wp-communication-health-check.mjs`
  - `node --check .GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
  - `node --check .GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `node .GOV/roles_shared/checks/gov-check.mjs`
  - `just build-order-sync`
  - `just gov-check`

### 2026.04.26.7 / GOV-CHANGE-20260426-07

- STATUS: APPLIED
- SUMMARY: queued closeout canonicalization refactor items and implementation briefs
- CHANGE_TYPE: GOVERNANCE_PLANNING
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: closeout repair loops still read and rewrite governance artifacts instead of relying on one canonical terminal state, causing token waste, repair loops, and progress stalls.
  - `RGF-233`
  - `RGF-234`
  - `RGF-235`
  - `RGF-236`
  - `RGF-237`
  - `RGF-238`
  - `RGF-239`
  - `RGF-240`
  - `RGF-241`
- FOLLOW_ON_ITEMS:
  - `RGF-233`
  - `RGF-234`
  - `RGF-235`
  - `RGF-236`
  - `RGF-237`
  - `RGF-238`
  - `RGF-239`
  - `RGF-240`
  - `RGF-241`
- FILES_CHANGED:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_BRIEFS_20260426.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: The governance refactor board now queues `RGF-233` through `RGF-241` for canonical terminal closeout records, product-proof/projection-sync separation, product-only main compatibility, terminal session settlement, closeout debt reports, repair-loop budgets, legacy terminal migration, monotonic terminal publication, and executable closeout breakpoint scenarios.
- VERIFICATION:
  - `just build-order-sync`
  - `just gov-check`

### 2026.04.26.6 / GOV-CHANGE-20260426-06

- STATUS: APPLIED
- SUMMARY: added a mechanical single-authority guard to visible Orchestrator rescue
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: backup Orchestrator launch must not create competing Orchestrators
  - `RGF-232`
- FOLLOW_ON_ITEMS: []
- FILES_CHANGED:
  - `.GOV/roles/orchestrator/scripts/orchestrator-rescue.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-rescue-lib.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-rescue.test.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-telemetry-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-telemetry-lib.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: `just orchestrator-rescue [WP-{ID}]` now evaluates downtime/control-plane freshness before generating the visible session, prints the guard decision, records `ORCHESTRATOR_TAKEOVER_ATTEMPT` for non-dry-run WP-scoped rescue launches, and defaults the generated prompt to read-only/status mode unless downtime red-alert criteria or explicit `--force-takeover` Operator authority permits takeover. Takeover attempts are projected as Orchestrator push alerts so operator/status surfaces can see the backup launch.
- VERIFICATION:
  - `node --check .GOV/roles/orchestrator/scripts/orchestrator-rescue-lib.mjs`
  - `node --check .GOV/roles/orchestrator/scripts/orchestrator-rescue.mjs`
  - `node --test .GOV/roles/orchestrator/tests/orchestrator-rescue.test.mjs .GOV/roles_shared/tests/session-telemetry-lib.test.mjs .GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `just orchestrator-rescue WP-1-Calendar-Sync-Engine-v3 --dry-run`
  - `just gov-check`

### 2026.04.26.5 / GOV-CHANGE-20260426-05

- STATUS: APPLIED
- SUMMARY: added Orchestrator downtime red alert projection to the relay watchdog
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: ACP should raise a red alert when Orchestrator/control-plane downtime exceeds 10 minutes and recommend visible rescue at 20 minutes
  - `RGF-230`
- FOLLOW_ON_ITEMS:
  - `RGF-232`
- FILES_CHANGED:
  - `.GOV/roles/orchestrator/scripts/lib/orchestrator-downtime-alert-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-downtime-alert.test.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-telemetry-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-telemetry-lib.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: `just wp-relay-watchdog` now evaluates active orchestrator-managed WPs for stale Orchestrator/control-plane progress, ignores prior downtime alerts as fresh-progress evidence, emits deduped `RED_ALERT_ORCHESTRATOR_DOWNTIME` notifications to the Orchestrator lane at the 10-minute band, changes the 20-minute band recommendation to `just orchestrator-rescue WP-{ID}`, and makes active notification projection, `orchestrator-next`, plus push-alert telemetry stop on that red alert instead of continuing blind re-wakes.
- VERIFICATION:
  - `node --check .GOV/roles/orchestrator/scripts/lib/orchestrator-downtime-alert-lib.mjs`
  - `node --check .GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs`
  - `node --check .GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `node --check .GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `node --test .GOV/roles/orchestrator/tests/orchestrator-downtime-alert.test.mjs .GOV/roles/orchestrator/tests/orchestrator-next.test.mjs .GOV/roles_shared/tests/session-telemetry-lib.test.mjs .GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `just wp-relay-watchdog WP-1-Calendar-Sync-Engine-v3 --observe-only`
  - `just gov-check`

### 2026.04.26.4 / GOV-CHANGE-20260426-04

- STATUS: APPLIED
- SUMMARY: added the visible Orchestrator rescue launcher and fallback ladder
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: backup Orchestrator startup must open a visible interactive terminal, not a headless ACP lane
  - `RGF-228`
  - `RGF-229`
  - `RGF-231`
- FOLLOW_ON_ITEMS:
  - `RGF-230`
  - `RGF-232`
- FILES_CHANGED:
  - `justfile`
  - `.GOV/roles/orchestrator/scripts/orchestrator-rescue.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-rescue-lib.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-rescue.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: `just orchestrator-rescue [WP-{ID}]` now creates a visible takeover script under the temp directory, tries Windows Terminal first, visible PowerShell second, and leaves an exact manual script fallback. The generated session runs `orchestrator-health`, launches Codex on `gpt-5.5` with `model_reasoning_effort=xhigh`, embeds the Orchestrator rescue prompt, labels itself as the sanctioned visible-terminal exception, and avoids ACP headless launch mechanics.
- VERIFICATION:
  - `node --check .GOV/roles/orchestrator/scripts/orchestrator-rescue-lib.mjs`
  - `node --check .GOV/roles/orchestrator/scripts/orchestrator-rescue.mjs`
  - `node --test .GOV/roles/orchestrator/tests/orchestrator-rescue.test.mjs .GOV/roles/orchestrator/tests/orchestrator-health.test.mjs`
  - `just orchestrator-rescue WP-1-Calendar-Sync-Engine-v3 --dry-run`

### 2026.04.26.3 / GOV-CHANGE-20260426-03

- STATUS: APPLIED
- SUMMARY: added a compact read-only Orchestrator health bundle for recovery and takeover checks
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: one command should show ACP health, active roles, model/profile state, and WP lifecycle during recovery
  - `RGF-227`
- FOLLOW_ON_ITEMS:
  - `RGF-228`
  - `RGF-229`
  - `RGF-230`
  - `RGF-231`
  - `RGF-232`
- FILES_CHANGED:
  - `justfile`
  - `.GOV/roles/orchestrator/scripts/orchestrator-health.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-health-lib.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-health.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: `just orchestrator-health [WP-{ID}]` now prints a bounded read-only recovery bundle with registry freshness, ACP broker liveness/build/active-run state, WP lifecycle and steering blockers when a WP is supplied, role rows with model/profile/thread/command/queue/stale-age/worktree state, active runs, and the next safe `orchestrator-next` command. Unfiltered mode suppresses historical closed-session noise.
- VERIFICATION:
  - `node --check .GOV/roles/orchestrator/scripts/orchestrator-health-lib.mjs`
  - `node --check .GOV/roles/orchestrator/scripts/orchestrator-health.mjs`
  - `node --test .GOV/roles/orchestrator/tests/orchestrator-health.test.mjs`
  - `just orchestrator-health`
  - `just orchestrator-health WP-1-Calendar-Sync-Engine-v3`

### 2026.04.26.2 / GOV-CHANGE-20260426-02

- STATUS: APPLIED
- SUMMARY: replaced guessed permanent worktree paths with topology-resolved protected worktree authority
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: wrong `handshake_main` path guesses must not stop recovery/status commands without actionable diagnostics
  - `RGF-226`
- FOLLOW_ON_ITEMS:
  - `RGF-227`
  - `RGF-228`
  - `RGF-229`
  - `RGF-230`
  - `RGF-231`
  - `RGF-232`
- FILES_CHANGED:
  - `justfile`
  - `.GOV/roles_shared/scripts/topology/git-topology-lib.mjs`
  - `.GOV/roles_shared/scripts/topology/resolve-protected-worktree.mjs`
  - `.GOV/roles_shared/checks/docs-check.mjs`
  - `.GOV/roles_shared/scripts/protocol-ack.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles/orchestrator/scripts/role-session-worktree-add.mjs`
  - `.GOV/roles_shared/scripts/topology/sync-gov-to-main.mjs`
  - `.GOV/roles_shared/scripts/topology/gov-flush.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/tests/git-topology-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: protected worktree consumers now resolve `handshake_main`, `wt-gov-kernel`, and `wt-ilja` from live `git worktree list --porcelain` truth before falling back to configured sibling paths. `docs-check`, `gov-check`, `protocol-ack`, `sync-gov-to-main`, `gov-flush`, Integration Validator launch/start helpers, and the new `just resolve-protected-worktree` command surface path failures with discovered worktrees instead of leaving the operator with a silent or guessed-path failure.
- VERIFICATION:
  - `node --check` on modified topology/session/orchestrator scripts
  - `node --test .GOV/roles_shared/tests/git-topology-lib.test.mjs .GOV/roles_shared/tests/repomem-gate-cli.test.mjs`
  - `node --test .GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `node --test .GOV/roles/orchestrator/tests/session-launch-governance.test.mjs`
  - `node --test .GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
  - `just docs-check`
  - `node .GOV/roles_shared/scripts/topology/resolve-protected-worktree.mjs handshake_main --path-only`

### 2026.04.26.1 / GOV-CHANGE-20260426-01

- STATUS: APPLIED
- SUMMARY: allowed read-only Orchestrator status checks to continue without an active repomem session
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - 2026-04-26 Operator directive: `orchestrator-next` should not be blocked by missing repomem for read-only status and recovery checks
  - `RGF-225`
- FOLLOW_ON_ITEMS:
  - `RGF-226`
  - `RGF-227`
  - `RGF-228`
  - `RGF-229`
  - `RGF-230`
  - `RGF-231`
  - `RGF-232`
- FILES_CHANGED:
  - `justfile`
  - `.GOV/roles_shared/scripts/memory/repomem.mjs`
  - `.GOV/roles_shared/tests/repomem-gate-cli.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: `just orchestrator-next` now uses `repomem-soft-gate`, so a missing or stale repomem session emits a warning and read-only lifecycle/session truth can still print. Governed mutation commands continue to call hard `repomem-gate`, preserving the `repomem open` requirement before state-changing work.
- VERIFICATION:
  - `node --test .GOV/roles_shared/tests/repomem-gate-cli.test.mjs .GOV/roles_shared/tests/repomem-open-contract-lib.test.mjs .GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
  - `just repomem-soft-gate`

### 2026.04.25.3 / GOV-CHANGE-20260425-03

- STATUS: APPLIED
- SUMMARY: completed diagnostic Workflow Dossier write-lane separation and terminal repomem snapshot behavior
- CHANGE_TYPE: WORKFLOW_REDUCTION
- DRIVER_EVIDENCE:
  - 2026-04-25 Operator directive: dossier is malformed-tolerant diagnostic evidence, Orchestrator writes top-of-file, ACP writes EOF, terminal repomem snapshot appends after ACP settles, and Integration Validator FAIL should prefer same-WP remediation
  - `.GOV/Audits/smoketest/DOSSIER_20260425_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md`
- FOLLOW_ON_ITEMS:
  - `RGF-222`
  - `RGF-223`
  - `RGF-224`
- FILES_CHANGED:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier-lib.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md`
  - `.GOV/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/templates/WORKFLOW_DOSSIER_TEMPLATE.md`
  - `.GOV/roles/orchestrator/README.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
  - `.GOV/roles_shared/docs/TOOLING_GUARDRAILS.md`
  - `.GOV/roles_shared/tests/workflow-dossier-lib.test.mjs`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
- OUTCOME: Workflow Dossier writes now use separate diagnostic lanes: Orchestrator notes prepend near the top, ACP/session-control traces append at EOF, and closeout imports a full WP-bound repomem snapshot at EOF. Dossier append/import failures surface as `DIAGNOSTIC_DEBT` and no longer fail the closeout step. Integration Validator FAIL guidance now defaults to same-WP remediation unless scope expansion or explicit Operator choice requires a new WP.
- VERIFICATION:
  - `node --check .GOV\roles_shared\scripts\audit\workflow-dossier-lib.mjs; node --check .GOV\roles_shared\scripts\audit\workflow-dossier.mjs; node --check .GOV\roles_shared\scripts\audit\generate-post-run-audit-skeleton.mjs; node --check .GOV\roles_shared\checks\phase-check.mjs; node --check .GOV\roles\orchestrator\scripts\session-control-command.mjs`
  - `node --test .GOV\roles_shared\tests\workflow-dossier-lib.test.mjs`
  - `node --test .GOV\roles_shared\tests\phase-check.test.mjs`
  - `node --test .GOV\roles_shared\tests\generate-post-run-audit-skeleton.test.mjs`
  - `node .GOV\roles_shared\checks\task-board-check.mjs`
  - `just docs-check`
  - `just canonise-gov`
  - `just gov-check`
  - `git diff --check -- <refactor-touched files>`; repo-wide `git diff --check` still reports pre-existing trailing whitespace in the live Calendar Sync dossier outside this refactor

### 2026.04.25.2 / GOV-CHANGE-20260425-02

- STATUS: APPLIED
- SUMMARY: completed RGF-218 sparse repomem event contract and closeout memory import
- CHANGE_TYPE: WORKFLOW_REDUCTION
- DRIVER_EVIDENCE:
  - `AUDIT-20260421-CALENDAR-SYNC-ENGINE-SMOKETEST-REVIEW`
  - `SMOKETEST-REVIEW-20260421-CALENDAR-SYNC-ENGINE`
  - `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md`
- FOLLOW_ON_ITEMS:
  - `RGF-218`
- FILES_CHANGED:
  - `.GOV/roles_shared/scripts/memory/repomem-coverage-lib.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier-lib.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md`
  - `.GOV/templates/WORKFLOW_DOSSIER_TEMPLATE.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
- OUTCOME: governed roles now use sparse WP-bound repomem as the durable mid-run judgment trail, repomem coverage includes Classic Orchestrator and Activation Manager, Workflow Dossier sync remains mechanical telemetry only, and `phase-check CLOSEOUT` explicitly imports WP-bound repomem entries while excluding unrelated global session memories from parallel WPs.
- VERIFICATION:
  - `node --test .GOV\roles_shared\tests\repomem-coverage-lib.test.mjs .GOV\roles_shared\tests\workflow-dossier-lib.test.mjs`
  - `node --test .GOV\roles_shared\tests\phase-check.test.mjs .GOV\roles_shared\tests\generate-post-run-audit-skeleton.test.mjs`
  - `node .GOV\roles_shared\checks\task-board-check.mjs`
  - `just gov-check`

### 2026.04.25.1 / GOV-CHANGE-20260425-01

- STATUS: APPLIED
- SUMMARY: completed the reduction-first blocker-authority tranche and added a terminal verdict settlement fence
- CHANGE_TYPE: WORKFLOW_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260421-CALENDAR-SYNC-ENGINE-SMOKETEST-REVIEW`
  - `SMOKETEST-REVIEW-20260421-CALENDAR-SYNC-ENGINE`
  - `.GOV/Audits/smoketest/DOSSIER_20260421_CALENDAR_SYNC_ENGINE_WORKFLOW_DOSSIER.md`
- FOLLOW_ON_ITEMS:
  - `RGF-217`
  - `RGF-219`
  - `RGF-220`
  - `RGF-221`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-execution-state-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-closeout-dependency-lib.mjs`
  - `.GOV/roles_shared/checks/bundled-check-runner-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: terminal `Validated (...)` verdict truth now fences stale route anchors, stale next-actor mirrors, and stale open-review residue so post-verdict support drift remains settlement debt instead of resurrecting a live review lane; closeout dependency surfaces distinguish product outcome blockers from governance debt; aggregate bundle checks attribute real subcheck failures without treating host-load timeouts as workflow truth; token-budget failures remain detailed diagnostic telemetry instead of continuation blockers.
- VERIFICATION:
  - `node --test .GOV/roles_shared/tests/wp-execution-state-lib.test.mjs`
  - `node --test .GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `node --test .GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
  - `node --test .GOV/roles_shared/tests/wp-closeout-dependency-lib.test.mjs .GOV/roles_shared/tests/closeout-blocking-authority-lib.test.mjs`
  - `node --test .GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs .GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs`
  - `node --check .GOV/roles_shared/checks/bundled-check-runner-lib.mjs; node .GOV/roles_shared/checks/spec-bundle-check.mjs; node .GOV/roles_shared/checks/wp-comm-bundle-check.mjs; node .GOV/roles_shared/checks/topology-bundle-check.mjs`

### 2026.04.14.1 / GOV-CHANGE-20260414-01

- STATUS: APPLIED
- SUMMARY: hardened operator-gate handoff so approval/signature blockers emit durable machine notifications immediately on governed session settlement
- CHANGE_TYPE: WORKFLOW_HARDENING
- DRIVER_EVIDENCE:
  - `Operator directive 2026-04-14`
  - `WP-1-Distillation-v2 pre-signature handoff miss`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles/orchestrator/scripts/lib/operator-gate-notification-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles/orchestrator/tests/operator-gate-notification-lib.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- OUTCOME: the governed control plane now emits an idempotent `OPERATOR_GATE` notification as soon as a settled session lands in an operator-required blocker state, logs that event into the workflow dossier trail, and the operator viewport surfaces those waits as explicit operator action instead of generic notification debt. No governed role is restarted, interrupted, or auto-steered by this path.

### 2026.03.25.1 / GOV-CHANGE-20260325-01

- STATUS: APPLIED
- SUMMARY: enforced chat-visible refinement display as a hard Orchestrator requirement
- CHANGE_TYPE: POLICY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
- SURFACES:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
- OUTCOME: invisible terminal or tool output no longer counts as refinement-display proof

### 2026.03.25.2 / GOV-CHANGE-20260325-02

- STATUS: APPLIED
- SUMMARY: restricted Orchestrator helper-agent use so product-code writes require explicit operator approval and packet evidence
- CHANGE_TYPE: POLICY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
- SURFACES:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
- OUTCOME: governance/spec/refinement assistance remains allowed, but Orchestrator-managed product-code delegation is blocked unless operator approval is recorded explicitly

### 2026.03.25.3 / GOV-CHANGE-20260325-03

- STATUS: APPLIED
- SUMMARY: created stable smoketest linkage and governance-maintenance tracking surfaces for post-refactor follow-on work
- CHANGE_TYPE: RECORDKEEPING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
  - `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_SCHEMA_REGISTRY_V4_SMOKETEST_RECOVERY_REVIEW.md`
- FOLLOW_ON_ITEMS:
  - `RGF-03`
  - `RGF-04`
  - `RGF-05`
  - `RGF-06`
- OUTCOME: governance remediation now tracks by stable item IDs and changeset IDs instead of improvised Work-Packet-like handling

### 2026.03.25.4 / GOV-CHANGE-20260325-04

- STATUS: APPLIED
- SUMMARY: added a dedicated contract-hardening smoketest closeout review and upgraded the smoketest template to capture lineage, runtime truth, and merge containment explicitly
- CHANGE_TYPE: RECORDKEEPING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260325-CONTRACT-HARDENING-V1`
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
- SURFACES:
  - `.GOV/Audits/smoketest/AUDIT_20260325_CONTRACT_HARDENING_V1_SMOKETEST_CLOSEOUT_REVIEW.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
- OUTCOME: future smoketest reviews now have a stronger default structure for predecessor linkage, failure inventory, role review, ACP-runtime truth, remaining adjacent debt, and command-log evidence

### 2026.03.25.5 / GOV-CHANGE-20260325-05

- STATUS: APPLIED
- SUMMARY: made predecessor-to-successor improvement comparison a required smoketest-review element
- CHANGE_TYPE: REVIEW_DISCIPLINE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260325-CONTRACT-HARDENING-V1`
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
- SURFACES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_CONTRACT_HARDENING_V1_SMOKETEST_CLOSEOUT_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: follow-on smoketest reviews must now state exactly what improved relative to the previous smoketest, which makes recovery and closeout runs easier to compare and harder to overstate

### 2026.03.25.6 / GOV-CHANGE-20260325-06

- STATUS: APPLIED
- SUMMARY: required explicit `Handshake (Product)` versus `Repo Governance` scope splits in operator-facing chat and role guidance
- CHANGE_TYPE: OPERATOR_UX_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SURFACES:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/START_HERE.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `../handshake_main/AGENTS.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: operator-facing reasoning now distinguishes product code/spec/WP work from repo-governance/ACP/protocol work, even when the domain language is governance-themed

### 2026.03.25.7 / GOV-CHANGE-20260325-07

- STATUS: APPLIED
- SUMMARY: formalized a mandatory post-smoketest improvement rubric for workflow smoothness, Master Spec gap reduction, and token-cost pressure
- CHANGE_TYPE: REVIEW_DISCIPLINE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
  - `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- SURFACES:
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_ORCHESTRATOR_MANAGED_WP_WORKFLOW_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: future smoketest reviews must now state whether the workflow got smoother, whether the real Master Spec gap list got smaller, and whether the run got cheaper in operator/token cost, with a named next structural fix for each target

### 2026.03.25.8 / GOV-CHANGE-20260325-08

- STATUS: APPLIED
- SUMMARY: expanded smoketest reviews and live role guidance to treat silent failures, wrong command usage, ambiguity, and governance-document churn as explicit workflow signals
- CHANGE_TYPE: REVIEW_DISCIPLINE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
  - `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- SURFACES:
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_ORCHESTRATOR_MANAGED_WP_WORKFLOW_REVIEW.md`
- OUTCOME: future reviews and role guidance now treat false greens, wrong tool-family choices, ambiguous task/path truth, and repeated governance-document or command-surface rereads as explicit evidence of workflow ambiguity and avoidable token burn

### 2026.03.25.9 / GOV-CHANGE-20260325-09

- STATUS: APPLIED
- SUMMARY: moved minimal live-read-set and anti-rediscovery guidance into governed startup prompts and made the smoketest template more self-sufficient
- CHANGE_TYPE: TOKEN_DISCIPLINE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
  - `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: coder, validator, and integration-validator startup prompts now carry the minimal live read set directly, repeated protocol rereads or command rediscovery are explicitly treated as ambiguity signals, and the smoketest template remains usable even when the separate rubric document is not open

### 2026.03.25.10 / GOV-CHANGE-20260325-10

- STATUS: APPLIED
- SUMMARY: added the remaining post-smoketest workflow concerns as sequenced governance items after `RGF-06`
- CHANGE_TYPE: PLANNING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
  - `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-09`
  - `RGF-10`
  - `RGF-11`
  - `RGF-12`
- OUTCOME: the remaining concerns around invalidity rules, undeclared auxiliary worktrees, mid-run approval relapse, and signed-scope/current-main compatibility now exist as explicit sequenced governance work instead of only audit prose

### 2026.03.25.11 / GOV-CHANGE-20260325-11

- STATUS: APPLIED
- SUMMARY: split validator PASS closure from main containment so `Done` becomes merge-pending and `Validated (PASS)` requires local `main` containment proof
- CHANGE_TYPE: WORKFLOW_TRUTH_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SURFACES:
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles_shared/scripts/lib/merge-progression-truth-lib.mjs`
  - `.GOV/roles_shared/checks/merge-progression-truth-check.mjs`
  - `.GOV/roles_shared/checks/packet-truth-check.mjs`
  - `.GOV/roles_shared/checks/task-board-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles/orchestrator/scripts/task-board-set.mjs`
  - `.GOV/roles_shared/records/TASK_BOARD.md`
- OUTCOME: packets on the new format can no longer claim `Validated (PASS)` until the approved closure commit is recorded and proven to be contained in local `main`; Task Board and runtime truth now distinguish `[MERGE_PENDING]` from `[VALIDATED]`

### 2026.03.25.12 / GOV-CHANGE-20260325-12

- STATUS: APPLIED
- SUMMARY: added an integration-validator closeout preflight so final PASS commit clearance fails if the final lane cannot resolve the committed target or if WP-scoped session-control truth is still unsettled
- CHANGE_TYPE: FINAL_LANE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs`
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `justfile`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: orchestrator-managed final review now has a machine-checked preflight for topology safety and WP-scoped atomic closeout, and PASS commit clearance fails before partial closure truth can be written when that preflight is not satisfied

### 2026.03.25.13 / GOV-CHANGE-20260325-13

- STATUS: APPLIED
- SUMMARY: added deterministic session-control self-settlement so recoverable orphaned requests gain exactly one terminal result row without manual ledger repair
- CHANGE_TYPE: ACP_RUNTIME_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-self-settle-lib.mjs`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles_shared/tests/session-control-self-settle-lib.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: broker startup and governed session-control helpers now auto-recover missing terminal result rows for recoverable orphan cases, which reduces manual runtime truth repair and makes `SESSION_CONTROL_REQUESTS` -> `SESSION_CONTROL_RESULTS` convergence machine-driven

### 2026.03.25.14 / GOV-CHANGE-20260325-14

- STATUS: APPLIED
- SUMMARY: added explicit modeled lineage for packets that are both failed historical closures and live smoketest baselines
- CHANGE_TYPE: WORKFLOW_TRUTH_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
  - `.GOV/roles_shared/records/TASK_BOARD.md`
  - `.GOV/roles_shared/scripts/lib/historical-smoketest-lineage-lib.mjs`
  - `.GOV/roles_shared/checks/historical-smoketest-lineage-check.mjs`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: historical failed packets can now remain visible as live smoketest lineage anchors without overloading stub/backlog or superseded truth, and `gov-check` enforces the modeled linkage

### 2026.03.25.15 / GOV-CHANGE-20260325-15

- STATUS: APPLIED
- SUMMARY: added machine-visible orchestrator-managed workflow invalidity receipts and blocked manual checkpoint relapse on those lanes
- CHANGE_TYPE: WORKFLOW_INVALIDITY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-invalidity-flag.mjs`
  - `.GOV/roles_shared/schemas/WP_RECEIPT.schema.json`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles/coder/checks/pre-work.mjs`
  - `.GOV/roles/coder/checks/post-work-check.mjs`
  - `.GOV/roles/coder/scripts/coder-next.mjs`
  - `.GOV/roles/coder/checks/coder-skeleton-checkpoint.mjs`
  - `.GOV/roles_shared/checks/skeleton-approved.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: orchestrator-managed WPs now emit a structured `WORKFLOW_INVALIDITY` state for invalid procedure, communication-health and validator closeout can fail on that ledgered state, and manual skeleton checkpoint/approval commands are no longer silently tolerated on those lanes

### 2026.03.25.16 / GOV-CHANGE-20260325-16

- STATUS: APPLIED
- SUMMARY: enforced packet-declared WP topology and rejected undeclared auxiliary worktrees during live checks and closeout
- CHANGE_TYPE: TOPOLOGY_TRUTH_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-declared-topology-lib.mjs`
  - `.GOV/roles_shared/checks/worktree-concurrency-check.mjs`
  - `.GOV/roles_shared/checks/wp-declared-topology-check.mjs`
  - `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles_shared/tests/wp-declared-topology-lib.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_WORKTREES.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `justfile`
- OUTCOME: auxiliary detached check/postwork/validator clones are now mechanically visible as topology violations, the global concurrency gate and final validator gates share the same topology law, and `just wp-declared-topology-check WP-{ID}` exposes the packet-declared topology for one WP directly

### 2026.03.26.01 / GOV-CHANGE-20260326-01

- STATUS: APPLIED
- SUMMARY: blocked routine post-signature Operator interruption on orchestrator-managed lanes and made resume outputs name blocker classes explicitly
- CHANGE_TYPE: OPERATOR_AUTONOMY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles_shared/scripts/lib/role-resume-utils.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed role startup prompts now forbid routine post-signature Operator approval/proceed/checkpoint relapse on orchestrator-managed lanes, resume output carries machine-visible `BLOCKER_CLASS` state, and protocol alignment checks fail if that contract drifts

### 2026.03.26.02 / GOV-CHANGE-20260326-02

- STATUS: APPLIED
- SUMMARY: added signed-scope compatibility truth and blocked ad hoc packet widening during final-lane closeout
- CHANGE_TYPE: SCOPE_GOVERNANCE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs`
  - `.GOV/roles_shared/tests/signed-scope-compatibility-lib.test.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: modern packets now carry explicit current-`main` compatibility and packet-widening truth, final-lane closeout fails on stale compatibility baselines or ungoverned adjacent scope drift, and new packets default to the stricter 2026-03-26 packet format

### 2026.03.26.03 / GOV-CHANGE-20260326-03

- STATUS: APPLIED
- SUMMARY: added terminal closeout projection sync so task-board terminal moves reject packet drift and immediately refresh runtime projection truth
- CHANGE_TYPE: CLOSEOUT_TRUTH_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `.GOV/roles/orchestrator/scripts/task-board-set.mjs`
  - `.GOV/roles_shared/scripts/lib/merge-progression-truth-lib.mjs`
  - `.GOV/roles_shared/tests/merge-progression-truth-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: terminal Task Board moves now fail if packet status disagrees with the requested board state, runtime projection fields are refreshed immediately from packet truth on those transitions, and merge-progression checks now detect lagging runtime `current_packet_status`

### 2026.03.26.04 / GOV-CHANGE-20260326-04

- STATUS: APPLIED
- SUMMARY: added a canonical integration-validator context bundle so final-lane review can open one authority/path/source-of-truth surface instead of rediscovering final-lane context manually
- CHANGE_TYPE: FINAL_LANE_CONTEXT_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/validator/checks/integration-validator-context-brief.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
  - `.GOV/roles/validator/tests/validator-command-surface.test.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/validator/README.md`
  - `justfile`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: the Integration Validator now has a single read-only context bundle command that surfaces authority, actor/session identity, committed handoff target, declared topology, current-`main` compatibility, and the exact final-lane command sequence, which reduces repeated protocol rereads and path/authority rediscovery

### 2026.03.26.05 / GOV-CHANGE-20260326-05

- STATUS: APPLIED
- SUMMARY: added a dedicated operator-rule-restatement invalidity helper and projected those cases to a machine-visible lane-reset route
- CHANGE_TYPE: WORKFLOW_INVALIDITY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/wp/wp-operator-rule-restatement.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `justfile`
- OUTCOME: when the Operator has to restate a core orchestrator-managed lane rule, the repo now has a dedicated helper that records the condition as `OPERATOR_RULE_RESTATEMENT`, and communication/runtime projection routes that WP to `LANE_RESET_REQUIRED` instead of treating it as generic invalidity noise

### 2026.03.26.06 / GOV-CHANGE-20260326-06

- STATUS: APPLIED
- SUMMARY: narrowed validator gate write surfaces so wrong-lane orchestrator-managed usage fails early and points to the correct helper family
- CHANGE_TYPE: COMMAND_SURFACE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/tests/validator-next.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: `validator-gate-*` writes on orchestrator-managed packets now fail before mutating state when the current branch/worktree does not resolve to a governed validator lane, and the error points callers to `validator-next`, `integration-validator-context-brief`, or `external-validator-brief` instead of letting wrong-tool attempts masquerade as legitimate gate progression

### 2026.03.31.01 / GOV-CHANGE-20260331-01

- STATUS: APPLIED
- SUMMARY: restored communications-repair command-surface parity and made WP communications template drift fail closed before invalid runtime artifacts can be written
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - `.GOV/roles_shared/tests/ensure-wp-communications.test.mjs`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-command-surface.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-40`
- OUTCOME: the sanctioned `just ensure-wp-communications WP-{ID}` repair helper now exists, unreplaced `{{TOKEN}}` drift in packet communication templates is rejected before file writes, and regression checks cover both command-surface parity and template-token completeness

### 2026.03.31.02 / GOV-CHANGE-20260331-02

- STATUS: APPLIED
- SUMMARY: hardened orchestrator-managed launch so the ordinary launch path auto-issues the first governed ACP start instead of stopping at a launch-only false green
- CHANGE_TYPE: ACP_RUNTIME_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-40`
- OUTCOME: supported `launch-*` paths now auto-attempt the initial governed `START_SESSION`, startup proof becomes visible in launch summaries, and the explicit `start-*` helpers remain available as recovery tools instead of being required for the normal orchestrator-managed path

### 2026.04.01.01 / GOV-CHANGE-20260401-01

- STATUS: APPLIED
- SUMMARY: converged review-receipt projection so validator review traffic updates packet/task-board/build-order truth and runtime lifecycle state without manual repair
- CHANGE_TYPE: REVIEW_PROJECTION_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: negative `VALIDATOR_REVIEW` receipts now route back into coder remediation instead of falsely progressing toward final review, newer unresolved handoffs take precedence over older resolved review pairs, and review-driven packet/task-board/build-order/runtime truth stays converged after live direct-review traffic

### 2026.04.01.02 / GOV-CHANGE-20260401-02

- STATUS: APPLIED
- SUMMARY: added orchestrator governance-checkpoint notifications on validator-authored assessment receipts so workflow authority stays in the loop after each review decision
- CHANGE_TYPE: REVIEW_NOTIFICATION_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: validator-authored assessment receipts in orchestrator-managed lanes now emit a dedicated `ORCHESTRATOR` notification alongside the normal direct-review target, so the orchestrator can verify packet/runtime/task-board truth and ACP steering immediately after each assessment without taking over coder-validator communication

### 2026.04.01.03 / GOV-CHANGE-20260401-03

- STATUS: APPLIED
- SUMMARY: aligned `validator-next` and `orchestrator-next` with projected receipt/runtime truth so live review routes and validator assessment results surface mechanically instead of through stale packet wording
- CHANGE_TYPE: RESUME_SURFACE_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: validator assessment checkpoints now carry `PASS`/`FAIL`/`ASSESSED` plus the validator's reason, `validator-next` follows projected `next_expected_actor` / `waiting_on` truth before falling back to legacy packet wording, and `orchestrator-next` surfaces pending governance checkpoints with the validator result and steering/closeout follow-ons instead of silently treating them as background notifications

### 2026.04.01.04 / GOV-CHANGE-20260401-04

- STATUS: APPLIED
- SUMMARY: hardened the final validator lane so governed Integration Validator sessions always resolve live governance from the kernel and governed coder handoffs fail closed unless the PREPARE target is reviewable
- CHANGE_TYPE: FINAL_LANE_AND_HANDOFF_GATE_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/coder/scripts/lib/coder-governance-lib.mjs`
  - `.GOV/roles/coder/scripts/coder-next.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-review-exchange.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed Integration Validator launch/control now injects `HANDSHAKE_GOV_ROOT=<wt-gov-kernel>/.GOV`, final-lane closeout fails if live authority resolves from `handshake_main/.GOV`, coder resume surfaces reflect validator remediation truth directly, and governed `CODER_HANDOFF` receipt appends reject dirty/non-reviewable PREPARE state instead of recording a false validator-ready handoff

### 2026.04.01.05 / GOV-CHANGE-20260401-05

- STATUS: APPLIED
- SUMMARY: tightened final-lane startup and steering prompts so Integration Validator sessions must open the kernel-governed context brief before rediscovering governance surfaces
- CHANGE_TYPE: FINAL_LANE_PROMPT_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed Integration Validator prompts now explicitly forbid manual authority rebuilds from `handshake_main/.GOV`, require `just integration-validator-context-brief WP-{ID}` ahead of broader resume work, and keep kernel governance truth mechanically in front of the final lane even when product execution stays rooted in `handshake_main`

### 2026.04.01.06 / GOV-CHANGE-20260401-06

- STATUS: APPLIED
- SUMMARY: hardened final-lane closeout sync so contained-main closure can refresh stale compatibility truth and accept only signed-surface-preserving harmonization in local main
- CHANGE_TYPE: FINAL_LANE_CLOSEOUT_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles_shared/scripts/lib/signed-scope-surface-lib.mjs`
  - `.GOV/roles_shared/tests/signed-scope-surface-lib.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed closeout sync now writes packet/runtime truth before the final completion gate evaluates it, stale recorded compatibility can be refreshed by the sync itself, contained-main PASS closure allows conflict-resolved local-main harmonization only when the resulting commit stays within the signed file surface and still satisfies the governed tripwire checks, and terminal closeout retires stale coder/WP-validator steerable sessions so session-governance truth converges with terminal packet state

### 2026.04.01.07 / GOV-CHANGE-20260401-07

- STATUS: APPLIED
- SUMMARY: blocked kernel-to-main governance sync on dirty kernel state so main-side sync provenance cannot silently reference stale kernel commits
- CHANGE_TYPE: SYNC_PROVENANCE_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/topology/sync-gov-to-main.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: `just sync-gov-to-main` now fails closed when `wt-gov-kernel/.GOV` has uncommitted changes, so `GOV_KERNEL_SYNC.json` and main-side governance sync commits always refer to committed kernel truth instead of mirroring an uncheckpointed governance snapshot under a stale kernel SHA

### 2026.04.01.08 / GOV-CHANGE-20260401-08

- STATUS: APPLIED
- SUMMARY: hardened final-lane boundary enforcement so wrong-lane closeout attempts emit governed invalidity and successful contained-main closeout leaves machine-readable provenance
- CHANGE_TYPE: FINAL_LANE_BOUNDARY_AND_PROVENANCE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/validator/checks/integration-validator-context-brief.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-47`
- OUTCOME: final-lane closeout now classifies role-boundary misuse into machine-visible `WORKFLOW_INVALIDITY` codes, kernel/orchestrator-side closeout misuse can no longer quietly become closure truth, and successful closeout sync records attributable provenance for contained-main promotion in validator gate state plus closeout receipts

### 2026.04.01.09 / GOV-CHANGE-20260401-09

- STATUS: APPLIED
- SUMMARY: added a contract-heavy intent checkpoint between `CODER_INTENT` and `CODER_HANDOFF` so governed direct-review lanes catch signed-surface/proof drift before full handoff
- CHANGE_TYPE: DIRECT_REVIEW_CHECKPOINT_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/coder/scripts/lib/coder-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: contract-heavy or under-specified intent now routes to `WP_VALIDATOR_INTENT_CHECKPOINT`, validator/coder resume helpers surface checkpoint-specific guidance, and governed `CODER_HANDOFF` fails closed until the checkpoint or any open review items are resolved so late-loop rework and false completion claims are reduced mechanically

### 2026.04.01.10 / GOV-CHANGE-20260401-10

- STATUS: APPLIED
- SUMMARY: captured new review-quality, microtask-discipline, and LLM-first data follow-ons from the WP-1 smoketest evidence so the governance board now maps them to concrete remediation work
- CHANGE_TYPE: SMOKETEST_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/Audits/smoketest/AUDIT_20260331_PROJECT_PROFILE_EXTENSION_REGISTRY_V1_SMOKETEST_STARTUP_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-48`
  - `RGF-49`
  - `RGF-50`
  - `RGF-51`
- OUTCOME: the smoketest review now explicitly records the operator requirement for validator-owned bootstrap/skeleton review, rolling microtask review overlap, zero-debt anti-vibe enforcement, and a codified LLM-first data doctrine, and the governance task board now exposes those concerns as concrete follow-on remediation items linked back to the same audit evidence

### 2026.04.01.11 / GOV-CHANGE-20260401-11

- STATUS: APPLIED
- SUMMARY: added validator-owned bootstrap/skeleton clearance plus bounded rolling overlap review so governed direct-review lanes catch weak intent earlier without letting microtask debt accumulate silently
- CHANGE_TYPE: EARLY_REVIEW_AND_MICROTASK_BACKPRESSURE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-review-exchange.mjs`
  - `.GOV/roles_shared/schemas/WP_RECEIPT.schema.json`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles_shared/tests/wp-communications-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles_shared/tests/protocol-alignment-check.test.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-50`
  - `RGF-51`
- OUTCOME: after every governed `CODER_INTENT`, the lane now waits for explicit WP-validator bootstrap/skeleton clearance instead of silently trusting coder intent; completed microtasks can be reviewed in bounded overlap mode with a hard queue limit of 2; overlap review backlog now creates mechanical backpressure; full `CODER_HANDOFF` fails closed until overlap review debt is drained; startup prompts, resume helpers, runtime route truth, and the operator command surface now all describe the same early-review law

### 2026.04.01.12 / GOV-CHANGE-20260401-12

- STATUS: APPLIED
- SUMMARY: codified the LLM-first data contract and anti-vibe zero-debt review law so new packets must prove data posture explicitly and PASS can no longer coexist with shallow signed-scope work
- CHANGE_TYPE: DATA_CONTRACT_AND_ZERO_DEBT_REVIEW_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/data-contract-lib.mjs`
  - `.GOV/roles_shared/tests/data-contract-lib.test.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles/coder/checks/post-work-check.mjs`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles/validator/tests/validator-report-structure-check.test.mjs`
  - `.GOV/roles/validator/docs/VALIDATOR_ANTI_GAMING_RUBRIC.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: new packets (`PACKET_FORMAT_VERSION >= 2026-04-01`) can declare `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, packet creation and claim-time checks now require an authoritative data-contract monitoring block, validator closure now requires explicit `DATA_CONTRACT_PROOF` and `DATA_CONTRACT_GAPS` for active data-contract packets, coder handoffs on the new packet format now include anti-vibe, signed-scope-debt, and data-contract self-check fields, and governed RIGOR_V3 PASS law now rejects unresolved anti-vibe findings or signed-scope debt on the new packet format instead of leaving those concerns as prose-only review style

### 2026.04.01.13 / GOV-CHANGE-20260401-13

- STATUS: APPLIED
- SUMMARY: captured `roles_shared` follow-on gaps after the new packet-law hardening so explicit data-contract activation, shared-doc alignment, and end-to-end regression coverage are tracked as concrete governance work
- CHANGE_TYPE: POST_HARDENING_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-52`
  - `RGF-53`
  - `RGF-54`
- OUTCOME: the governance board now tracks three `roles_shared` follow-ons that were left implicit after `RGF-50` and `RGF-51`: making data-contract activation explicit instead of keyword-inferred, aligning shared docs/operator-facing command surfaces with the new packet law, and adding an end-to-end regression plus explicit migration policy for older packet families

### 2026.04.01.14 / GOV-CHANGE-20260401-14

- STATUS: APPLIED
- SUMMARY: captured the next orchestrator/shared follow-on after spotting stale legacy refinement-path guidance in operator-facing next-command surfaces and shared docs
- CHANGE_TYPE: PATH_TRUTH_ALIGNMENT_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-55`
- OUTCOME: the governance board now tracks a dedicated path-truth alignment item for the post-layout-migration drift where orchestrator/operator/shared surfaces still hard-code the legacy flat `.GOV/refinements/WP-{ID}.md` path instead of resolving the current co-located packet/refinement layout or using path-neutral wording

### 2026.04.01.15 / GOV-CHANGE-20260401-15

- STATUS: APPLIED
- SUMMARY: captured the next orchestrator follow-on after confirming that the new `2026-04-01` packet-law bundle is enforced in code but still under-exposed in orchestrator-facing protocol and launch surfaces
- CHANGE_TYPE: OPERATOR_PACKET_LAW_VISIBILITY_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-56`
- OUTCOME: the governance board now tracks an orchestrator-specific visibility item so packet creation, next-command surfaces, and operator-facing protocol stop hiding the active `PACKET_FORMAT_VERSION`, `DATA_CONTRACT_PROFILE`, anti-vibe, signed-scope-debt, and validator-proof obligations that are already enforced later by coder/validator checks

### 2026.04.01.16 / GOV-CHANGE-20260401-16

- STATUS: APPLIED
- SUMMARY: captured the validator-side path-truth drift after finding a remaining hard-rule reference to the legacy flat refinement path in the validator protocol
- CHANGE_TYPE: VALIDATOR_PATH_TRUTH_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-57`
- OUTCOME: the governance board now tracks the remaining validator path-truth cleanup so refinement-completeness rules stop referencing only `.GOV/refinements/WP-{ID}.md` after the packet family migrated to co-located `task_packets/WP-{ID}/refinement.md`

### 2026.04.01.17 / GOV-CHANGE-20260401-17

- STATUS: APPLIED
- SUMMARY: captured the coder-side path-truth drift after finding legacy packet-read and refinement-read instructions still embedded in the coder protocol workflow
- CHANGE_TYPE: CODER_PATH_TRUTH_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-58`
- OUTCOME: the governance board now tracks the coder-side path-truth cleanup so pre-work guidance stops telling the coder to use legacy wildcard packet paths or only `.GOV/refinements/WP-{ID}.md` when the official packet family already supports co-located `packet.md` and `refinement.md`

### 2026.04.02.01 / GOV-CHANGE-20260402-01

- STATUS: APPLIED
- SUMMARY: completed `RGF-55` by removing stale hard-coded refinement-path wording from orchestrator outputs and aligning the shared path docs with the current folder packet family
- CHANGE_TYPE: PATH_TRUTH_ALIGNMENT_PATCH
- DRIVER_EVIDENCE:
  - `RGF-55`
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: operator-facing orchestrator guidance now references the resolved current refinement path instead of hard-coding `.GOV/refinements/WP-{ID}.md`, and the shared protocol/invariant docs now describe the folder packet/refinement layout as current truth while still naming the legacy flat layout explicitly as compatibility-only

### 2026.04.02.02 / GOV-CHANGE-20260402-02

- STATUS: APPLIED
- SUMMARY: completed `RGF-57` and `RGF-58` by aligning validator and coder protocol path guidance to the current folder packet family while preserving legacy flat compatibility wording
- CHANGE_TYPE: ROLE_PROTOCOL_PATH_TRUTH_ALIGNMENT_PATCH
- DRIVER_EVIDENCE:
  - `RGF-57`
  - `RGF-58`
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: validator refinement review now names the official current refinement path instead of only the flat legacy path, coder pre-work guidance now reads the current packet/refinement layout first and treats flat files as explicit compatibility-only paths, and the stale wildcard packet-read examples are removed from coder workflow instructions

### 2026.04.02.03 / GOV-CHANGE-20260402-03

- STATUS: APPLIED
- SUMMARY: completed `RGF-53` and `RGF-56` by surfacing the `2026-04-01` packet-law bundle in the shared operator docs and orchestrator packet-create/resume flow
- CHANGE_TYPE: PACKET_LAW_VISIBILITY_ALIGNMENT_PATCH
- DRIVER_EVIDENCE:
  - `RGF-53`
  - `RGF-56`
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/START_HERE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: shared/operator-facing docs now state that packet creation activates the `2026-04-01` law bundle rather than just scaffolding files, and orchestrator packet-create/resume output now surfaces the active packet format, data-contract posture, handoff/report rigor profile, anti-vibe and signed-scope-debt consequences, and data-contract proof obligations so new governed lanes do not start blind to the checks that will later enforce closure truth

### 2026.04.02.04 / GOV-CHANGE-20260402-04

- STATUS: APPLIED
- SUMMARY: completed `RGF-52` and `RGF-54` by making data-contract activation or waiver an explicit packet decision, enforcing it at claim and validator closeout, adding end-to-end regression coverage for the new packet family, and recording the grandfathered legacy packet-family compatibility surface explicitly
- CHANGE_TYPE: DATA_CONTRACT_DECISION_AND_PACKET_LAW_REGRESSION_PATCH
- DRIVER_EVIDENCE:
  - `RGF-52`
  - `RGF-54`
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/data-contract-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles_shared/tests/data-contract-lib.test.mjs`
  - `.GOV/roles_shared/tests/new-packet-law-regression.test.mjs`
  - `.GOV/roles/validator/tests/validator-report-structure-check.test.mjs`
  - `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`
  - `.GOV/roles_shared/records/COMPATIBILITY_SHIM_LEDGER.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: `PACKET_FORMAT_VERSION >= 2026-04-01` packets now carry an explicit `DATA_CONTRACT_DECISION` that must either activate the LLM-first data contract with reviewable evidence or explicitly waive it as not data-bearing; claim and validator closeout gates reject mismatched or conflicted waivers, regression coverage now proves the active-vs-waived-vs-grandfathered behavior end-to-end, and the older packet family remains explicitly tracked as an `ACTIVE_COMPAT` governance shim instead of an undocumented implicit exception

### 2026.04.02.05 / GOV-CHANGE-20260402-05

- STATUS: APPLIED
- SUMMARY: recorded and fixed the permanent-worktree reseed helper bug discovered during the `user_ilja` refresh sequence, where the helper failed on a checked-out target branch and then falsely treated expected `.GOV` junction drift as a dirty-worktree failure
- CHANGE_TYPE: TOPOLOGY_RESEED_HELPER_MAINTENANCE_CAPTURE
- DRIVER_EVIDENCE:
  - `RGF-59`
  - `MAINT-20260402-PERMANENT-WORKTREE-RESEED-HELPER`
- SURFACES:
  - `.GOV/roles_shared/scripts/topology/reseed-permanent-worktree-from-main.mjs`
  - `.GOV/roles_shared/scripts/topology/git-topology-lib.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: `just reseed-permanent-worktree-from-main wt-ilja ...` now updates the checked-out permanent branch safely with `checkout -B` semantics instead of trying to force-move the live branch ref, and the helper's cleanliness gate now checks for non-`.GOV` dirt so expected governance-junction replacement in permanent non-main worktrees does not cause a false failure after a successful reseed

### 2026.04.03.01 / GOV-CHANGE-20260403-01

- STATUS: APPLIED
- SUMMARY: captured new governance follow-ons from the storage-trait smoketest closeout review covering terminal runtime residue, pre-implementation packet validity, broker-safe closeout, and review-verdict deduplication
- CHANGE_TYPE: STORAGE_TRAIT_SMOKETEST_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/Audits/smoketest/AUDIT_20260403_STORAGE_TRAIT_PURITY_V1_SMOKETEST_CLOSEOUT_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-60`
  - `RGF-61`
  - `RGF-62`
  - `RGF-63`
- OUTCOME: the governance board now tracks the four highest-value follow-ons surfaced by the storage-trait closeout review, with `RGF-60` activated first because the current terminal runtime surface still carries stale residue after successful contained-main closure

### 2026.04.03.02 / GOV-CHANGE-20260403-02

- STATUS: APPLIED
- SUMMARY: completed `RGF-60` by cleaning terminal runtime projection residue during final-lane closeout and stamping validator-of-record truth from the governed lane
- CHANGE_TYPE: TERMINAL_CLOSEOUT_RUNTIME_RESIDUE_PATCH
- DRIVER_EVIDENCE:
  - `RGF-60`
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: terminal closeout now resolves validator-of-record values from governed receipts and the active final-lane actor, runtime projection writes those values into `RUNTIME_STATUS.json`, and `Validated (...)` packet states now clear stale `active_role_sessions` and touched-file residue instead of leaving misleading live-session artifacts behind after closure

### 2026.04.03.03 / GOV-CHANGE-20260403-03

- STATUS: APPLIED
- SUMMARY: completed `RGF-61` by making orchestrator-managed packet claim validity fail closed before implementation starts
- CHANGE_TYPE: PRE_IMPLEMENTATION_PACKET_CLAIM_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-61`
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles_shared/tests/new-packet-law-regression.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: orchestrator-managed packets with an assigned coder now leave creation with governed session-policy claim fields, and `READY_FOR_DEV` packets fail claim-check immediately if those fields are still unclaimed instead of surfacing only after implementation hardens

### 2026.04.03.04 / GOV-CHANGE-20260403-04

- STATUS: APPLIED
- SUMMARY: completed `RGF-62` by making final-lane closeout self-settle stale WP-scoped session-control state and tolerate only the current Integration Validator self-run
- CHANGE_TYPE: FINAL_LANE_CLOSEOUT_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-62`
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: closeout entrypoints now self-settle stale WP-scoped session-control rows before evaluation, and the closeout bundle no longer self-collides on the Integration Validator's own in-flight broker command while still failing on foreign or extra active runs

### 2026.04.03.05 / GOV-CHANGE-20260403-05

- STATUS: APPLIED
- SUMMARY: completed `RGF-63` by deduplicating decisive validator outcomes per review round and collapsing authoritative assessment surfaces
- CHANGE_TYPE: REVIEW_RECEIPT_DEDUPLICATION
- DRIVER_EVIDENCE:
  - `RGF-63`
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: duplicate decisive validator approvals or failures for the same review round now fail closed before they enter the receipt ledger, historical duplicate decisive assessments collapse into one authoritative surface for resume/projection consumers, and packet remediation text now points at the authoritative latest validator receipt instead of a raw noisy stream

### 2026.04.05.01 / GOV-CHANGE-20260405-01

- STATUS: APPLIED
- SUMMARY: completed Wave 1 (`RGF-64` through `RGF-67`) by finishing one-hop relay dispatch, typed route payloads, computed WP spans, and microtask-first resume surfaces
- CHANGE_TYPE: WORKFLOW_EFFICIENCY_AND_LANE_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-64`
  - `RGF-65`
  - `RGF-66`
  - `RGF-67`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs`
  - `.GOV/roles/orchestrator/scripts/lib/manual-relay-envelope-lib.mjs`
  - `.GOV/roles/coder/scripts/coder-next.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-timeline-report.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-microtask-lib.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/tests/wp-timeline-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-microtask-lib.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles/orchestrator/tests/manual-relay-envelope-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-68`
  - `RGF-69`
  - `RGF-70`
  - `RGF-71`
  - `RGF-72`
  - `RGF-73`
- OUTCOME: orchestrator-managed relay no longer requires a separate start turn and later steer turn for the same routine wakeup, governed prompts now carry typed route payloads instead of generic resume prose, `wp-timeline` computes control-command and review-exchange spans on top of the merged event stream, manual relay remains structured and first-class, continuation waivers are honored mechanically, and the compact role resume surfaces now expose declared active/next microtasks so coder and validator work can continue at MT granularity instead of broad WP guesswork

### 2026.04.05.02 / GOV-CHANGE-20260405-02

- STATUS: APPLIED
- SUMMARY: completed Wave 2 authority and validator hardening (`RGF-68` through `RGF-70`) by projecting milestone/task-board truth from shared helpers and upgrading validator law to `SPLIT_DIFF_SCOPED_RIGOR_V4`
- CHANGE_TYPE: AUTHORITY_REDUCTION_AND_VALIDATOR_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-68`
  - `RGF-69`
  - `RGF-70`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-05`
  - `SMOKE-FIND-20260404-06`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-authority-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/validator-report-profile-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/computed-policy-gate-lib.mjs`
  - `.GOV/roles_shared/checks/session-policy-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/templates/WP_RUNTIME_STATUS_TEMPLATE.json`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/ensure-wp-communications.test.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/computed-policy-gate-lib.test.mjs`
  - `.GOV/roles/validator/tests/validator-report-structure-check.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-71`
  - `RGF-72`
  - `RGF-74`
  - `RGF-75`
- OUTCOME: runtime and review projections now stamp derived milestone and task-board truth from one authority layer instead of scattered status mappers, contract-heavy direct-review lanes no longer stay pinned to stale bootstrap checkpoints once later validator proof exists, and new packets now default to `SPLIT_DIFF_SCOPED_RIGOR_V4`, forcing explicit primitive-retention, shared-surface, and current-main interaction evidence for stronger medium/high-risk closure audits

### 2026.04.05.03 / GOV-CHANGE-20260405-03

- STATUS: APPLIED
- SUMMARY: completed Wave 2 smoketest ledger hardening (`RGF-73`) by converting smoke reviews from narrative-only postmortems into stable finding and positive-control ledgers linked back to board items
- CHANGE_TYPE: RECORDKEEPING_AND_POSTMORTEM_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-73`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-07`
- SURFACES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
  - `.GOV/Audits/smoketest/AUDIT_20260404_PARALLEL_WP_ACP_STEERING_RECOVERY_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-71`
  - `RGF-72`
  - `RGF-74`
  - `RGF-75`
- OUTCOME: smoketest reviews now have stable `SMOKE-FIND-*` and `SMOKE-CONTROL-*` surfaces, board items can cite exact smoke findings instead of only whole audit documents, and the recovery audit now records both failure linkage and positive controls in a mechanically traceable format

### 2026.04.05.04 / GOV-CHANGE-20260405-04

- STATUS: APPLIED
- SUMMARY: completed Wave 3 hygiene and runtime-ownership hardening (`RGF-71`, `RGF-72`, `RGF-74`) by enforcing external artifact law, reclaiming registry-owned governed terminals, and moving work-packet naming into a compatibility-safe resolver layer
- CHANGE_TYPE: RUNTIME_HYGIENE_AND_PATH_AUTHORITY_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-71`
  - `RGF-72`
  - `RGF-74`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/scripts/lib/artifact-hygiene-lib.mjs`
  - `.GOV/roles_shared/scripts/topology/artifact-hygiene-check.mjs`
  - `.GOV/roles_shared/scripts/topology/artifact-cleanup.mjs`
  - `.GOV/roles/validator/checks/validator-git-hygiene.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles_shared/scripts/session/terminal-ownership-lib.mjs`
  - `.GOV/roles_shared/scripts/session/reclaim-owned-terminals.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles_shared/scripts/lib/runtime-paths.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-75`
- OUTCOME: repo-local `target/` directories and stale non-canonical artifact folders now fail closed and are cleaned mechanically, governed system-terminal sessions record ownership and can be reclaimed without touching unrelated operator terminals, and high-authority work-packet helpers/docs now resolve through `runtime-paths.mjs` with `work_packets` as the canonical logical name while legacy `.GOV/task_packets/` storage remains read-compatible during the migration window

### 2026.04.05.05 / GOV-CHANGE-20260405-05

- STATUS: APPLIED
- SUMMARY: closed `RGF-75` by evaluating branch topology after Wave 3 and confirming that `main` remains the only required stable product integration branch
- CHANGE_TYPE: TOPOLOGY_POLICY_EVALUATION
- DRIVER_EVIDENCE:
  - `RGF-75`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles_shared/docs/REPO_RESILIENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: after artifact enforcement and worktree-hygiene hardening, `handshake_main` is clean, no repo-local `target/` remains, and no additional stable product integration branch is justified; `main` stays canonical and branch-topology expansion remains closed unless future evidence shows `main` becoming operationally unsuitable again

### 2026.04.05.06 / GOV-CHANGE-20260405-06

- STATUS: APPLIED
- SUMMARY: registered the next governance follow-on tranche (`RGF-76` through `RGF-85`) so the post-wave roadmap is explicit before implementation begins
- CHANGE_TYPE: ROADMAP_AND_SEQUENCING_UPDATE
- DRIVER_EVIDENCE:
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-76`
  - `RGF-77`
  - `RGF-78`
  - `RGF-79`
  - `RGF-80`
  - `RGF-81`
  - `RGF-82`
  - `RGF-83`
  - `RGF-84`
  - `RGF-85`
- OUTCOME: the board now carries the next mechanical remediation set covering microtask-state hardening, full span ledgers, relay cost compression, dual-track validation, failure-ledger expansion, authority shrink, archival layout, legacy path cleanup, artifact retention, and session-batch terminal ownership

### 2026.04.05.07 / GOV-CHANGE-20260405-07

- STATUS: APPLIED
- SUMMARY: completed `RGF-76` by converting microtask handling from loose declared scope checks into a governed execution state machine with explicit active and previous microtask semantics
- CHANGE_TYPE: MICROTASK_EXECUTION_STATE_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-76`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-04`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-microtask-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles/coder/scripts/coder-next.mjs`
  - `.GOV/roles_shared/tests/wp-microtask-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-77`
  - `RGF-78`
  - `RGF-79`
  - `RGF-80`
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: kickoff-reviewed microtasks remain the active execution budget, overlap review shifts execution to the next declared microtask while retaining the previous reviewed slice explicitly, coder intent cannot jump ahead out of sequence, and validator overlap resolutions now fail closed unless they bind to the immediately previous governed microtask

### 2026.04.05.08 / GOV-CHANGE-20260405-08

- STATUS: APPLIED
- SUMMARY: completed `RGF-77` by upgrading `wp-timeline` from a compact report view into a richer span ledger with stage-tagged control, token, review, and microtask execution windows
- CHANGE_TYPE: TIMELINE_AND_COST_LEDGER_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-77`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-03`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-timeline-report.mjs`
  - `.GOV/roles_shared/tests/wp-timeline-lib.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-78`
  - `RGF-79`
  - `RGF-80`
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: `just wp-timeline` now emits stage-tagged span rows with stable span ids, explicit token-command windows, review-exchange durations, and microtask execution spans, plus summary counts for span families and measured span coverage so relay and delay hot spots are attributable without rereading scattered ledgers

### 2026.04.05.09 / GOV-CHANGE-20260405-09

- STATUS: APPLIED
- SUMMARY: completed `RGF-78` by attaching measured relay burden and default lane policy to the timeline surface, then surfacing the cheaper-lane guidance in the operator workflow docs and lane-selection gate output
- CHANGE_TYPE: RELAY_COST_POLICY_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-78`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-01`
  - `SMOKE-FIND-20260404-02`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-timeline-report.mjs`
  - `.GOV/roles_shared/tests/wp-timeline-lib.test.mjs`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-79`
  - `RGF-80`
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: `just wp-timeline` now reports relay command counts, relay token share, burden level, and a recommended lane, while signature-time operator guidance now states the default policy explicitly: use `MANUAL_RELAY` for small and medium WPs unless autonomous steering or multi-WP parallelism is clearly worth the extra prompt tax

### 2026.04.05.10 / GOV-CHANGE-20260405-10

- STATUS: APPLIED
- SUMMARY: completed `RGF-79` by making the validator split explicit for new medium/high V4 packets, with separate mechanical and spec-retention track verdicts enforced in both structure checks and computed closure policy
- CHANGE_TYPE: DUAL_TRACK_VALIDATOR_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-79`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-06`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/validator-report-profile-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `.GOV/roles_shared/scripts/lib/computed-policy-gate-lib.mjs`
  - `.GOV/roles_shared/tests/computed-policy-gate-lib.test.mjs`
  - `.GOV/roles/validator/tests/validator-report-structure-check.test.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-80`
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: new packets default to `PACKET_FORMAT_VERSION=2026-04-05`, creation/resume output now surfaces the dual-track law, and medium/high V4 validator closure must explicitly prove both the mechanical closure track and the deep spec-retention/shared-surface/current-main track before PASS remains legal

### 2026.04.05.11 / GOV-CHANGE-20260405-11

- STATUS: APPLIED
- SUMMARY: completed `RGF-80` by turning smoketest reviews into a typed failure/control ledger shape, then hardening the audit skeleton and maintenance workflow around that richer schema
- CHANGE_TYPE: FAILURE_LEDGER_SCHEMA_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-80`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-07`
- SURFACES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
  - `.GOV/roles_shared/tests/generate-post-run-audit-skeleton.test.mjs`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: new smoketest reviews now declare typed `CATEGORY`, `ROLE_OWNER`, `SYSTEM_SCOPE`, and `FAILURE_CLASS` fields plus typed positive controls with `CONTROL_TYPE`, `What went well`, and `REGRESSION_GUARDS`, and the post-run audit skeleton emits those placeholders by default instead of leaving the structure half-narrative

### 2026.04.05.12 / GOV-CHANGE-20260405-12

- STATUS: APPLIED
- SUMMARY: completed `RGF-81` by collapsing duplicated task-board status truth and repo-root work-packet path fallback logic into shared authority helpers, then rewiring resume/governance consumers to read those helpers instead of re-implementing local variants
- CHANGE_TYPE: AUTHORITY_SURFACE_SHRINK
- DRIVER_EVIDENCE:
  - `RGF-81`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-05`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-authority-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/runtime-paths.mjs`
  - `.GOV/roles_shared/scripts/session/session-governance-state-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/role-resume-utils.mjs`
  - `.GOV/roles_shared/tests/runtime-paths.test.mjs`
  - `.GOV/roles_shared/tests/session-governance-state-lib.test.mjs`
  - `.GOV/roles_shared/tests/role-resume-utils.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: task-board terminal and active-status truth plus repo-root work-packet/task-board resolution now come from shared helpers, so resume/governance surfaces stop carrying their own packet-path fallback and board-status regex copies

### 2026.04.05.13 / GOV-CHANGE-20260405-13

- STATUS: APPLIED
- SUMMARY: completed `RGF-83` by sweeping high-authority guidance, templates, and user-facing governance messages so they describe the logical `work_packets` model first and only mention `.GOV/task_packets/` as current physical compatibility storage
- CHANGE_TYPE: PATH_LANGUAGE_ALIGNMENT
- DRIVER_EVIDENCE:
  - `RGF-83`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`
  - `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
  - `.GOV/roles/validator/README.md`
  - `.GOV/roles_shared/docs/VALIDATOR_FILE_TOUCH_MAP.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/EVIDENCE_LEDGER.md`
  - `.GOV/roles_shared/docs/QUALITY_GATE.md`
  - `.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles_shared/checks/gate-check.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-traceability-set.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: the authoritative human-facing surfaces now teach â€œresolve Work Packets through the logical `work_packets` model, with `.GOV/task_packets/` as compatibility storageâ€ instead of presenting `task_packets` as the conceptual source of truth

### 2026.04.05.14 / GOV-CHANGE-20260405-14

- STATUS: APPLIED
- SUMMARY: completed `RGF-84` by turning governed artifact cleanup into a policy-backed retention workflow with durable manifests, so closeout now records exactly what residue was removed versus retained under `Handshake_Artifacts`
- CHANGE_TYPE: ARTIFACT_RETENTION_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-84`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/artifact-hygiene-lib.mjs`
  - `.GOV/roles_shared/scripts/topology/artifact-cleanup.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles_shared/tests/artifact-hygiene-lib.test.mjs`
  - `.GOV/roles_shared/docs/ARTIFACT_RETENTION_POLICY.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/REPO_RESILIENCE.md`
  - `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-82`
  - `RGF-85`
- OUTCOME: manual `artifact-cleanup` and integration-validator closeout now emit JSON retention manifests under `../Handshake_Artifacts/handshake-tool/artifact-retention/`, preserving durable cleanup evidence while keeping canonical artifact roots and non-reclaimable residue out of the auto-delete set

### 2026.04.05.15 / GOV-CHANGE-20260405-15

- STATUS: APPLIED
- SUMMARY: completed `RGF-82` by introducing a governed Work Packet lifecycle layout with reserved archive roots and archive-aware resolver support, without migrating existing packets or breaking active-path compatibility
- CHANGE_TYPE: WORK_PACKET_LIFECYCLE_LAYOUT
- DRIVER_EVIDENCE:
  - `RGF-82`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/runtime-paths.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs`
  - `.GOV/roles_shared/tests/runtime-paths.test.mjs`
  - `.GOV/roles_shared/docs/WORK_PACKET_LIFECYCLE_LAYOUT.md`
  - `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/task_packets/_archive/README.md`
  - `.GOV/task_packets/_archive/superseded/README.md`
  - `.GOV/task_packets/_archive/validated_closed/README.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-85`
- OUTCOME: the runtime-path resolver now understands reserved archive roots for `superseded` and `validated_closed` packets, packet creation ensures the lifecycle layout exists, and the repo carries explicit archive directories plus policy docs without forcing a risky bulk packet move

### 2026.04.05.16 / GOV-CHANGE-20260405-16

- STATUS: APPLIED
- SUMMARY: completed `RGF-85` by grouping registry-owned governed terminals under explicit terminal batch ids so closeout and manual reclaim can target only the intended governed run
- CHANGE_TYPE: TERMINAL_BATCH_OWNERSHIP_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-85`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `Justfile`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles_shared/scripts/session/terminal-ownership-lib.mjs`
  - `.GOV/roles_shared/scripts/session/reclaim-owned-terminals.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles/orchestrator/scripts/session-reset-batch-launch-mode.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles_shared/tests/session-registry-lib.test.mjs`
  - `.GOV/roles_shared/tests/terminal-ownership-lib.test.mjs`
  - `.GOV/roles_shared/schemas/ROLE_SESSION_REGISTRY.schema.json`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed system-terminal launches now stamp `owned_terminal_batch_id` plus the active registry batch, manual reclaim defaults to `CURRENT_BATCH` and can optionally target `ALL_BATCHES` or one explicit `BATCH_ID`, and operator-visible status surfaces now expose the active batch id so only the intended governed terminal set is reclaimed

### 2026.04.05.17 / GOV-CHANGE-20260405-17

- STATUS: APPLIED
- SUMMARY: completed `RGF-86` by introducing an explicit per-role model-profile catalog into packet/stub law, launch/session-control enforcement, and claim/session policy checks
- CHANGE_TYPE: ROLE_MODEL_PROFILE_CATALOG
- DRIVER_EVIDENCE:
  - `RGF-86`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles_shared/checks/session-policy-check.mjs`
  - `.GOV/roles/coder/checks/pre-work-check.mjs`
  - `.GOV/roles_shared/schemas/ROLE_SESSION_REGISTRY.schema.json`
  - `.GOV/roles_shared/schemas/SESSION_CONTROL_REQUEST.schema.json`
  - `.GOV/roles_shared/schemas/SESSION_LAUNCH_REQUEST.schema.json`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-87`
- OUTCOME: new packet families now record explicit role-model-profile ids, GPT remains the governed default, Claude Code Opus 4.6 Thinking Max is declared and auditable at packet level, and governed launch/session control fail closed instead of silently pretending unsupported provider runtime exists

### 2026.04.08.1 / GOV-CHANGE-20260408-01

- STATUS: APPLIED
- SUMMARY: completed `RGF-148` and `RGF-149` by adding stable phase-level startup/handoff/verdict/closeout entrypoints and switching role-facing guidance to the composite communication gates
- CHANGE_TYPE: PHASE_GATE_COMMAND_SURFACE_CONSOLIDATION
- DRIVER_EVIDENCE:
  - `RGF-148`
  - `RGF-149`
  - `AUDIT-20260408-PARALLEL-WP-CRASH-RECOVERY-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260408-PARALLEL-WP-CRASH-RECOVERY-CLOSEOUT`
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/tests/governance-command-contract.test.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-150`
  - `RGF-151`
- OUTCOME: governed roles now enter startup and final review through stable phase-level commands, startup mesh checks include the integration-validator peer when that lane is active, validator helper guidance points at the composite gates instead of raw low-level checks, and the next governance slice is reduced to single-writer lifecycle truth plus superseded-review projection compaction

### 2026.04.08.2 / GOV-CHANGE-20260408-02

- STATUS: APPLIED
- SUMMARY: completed `RGF-150` and `RGF-151` by moving review-stage lifecycle sync onto one reconciliation path and defaulting notification surfaces to active blocking routes
- CHANGE_TYPE: LIFECYCLE_TRUTH_SINGLE_WRITER_AND_ACTIVE_REVIEW_PROJECTION
- DRIVER_EVIDENCE:
  - `RGF-150`
  - `RGF-151`
  - `AUDIT-20260408-PARALLEL-WP-CRASH-RECOVERY-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260408-PARALLEL-WP-CRASH-RECOVERY-CLOSEOUT`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-check-notifications.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-check-notifications.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - Repair stale terminal `INTEGRATION_VALIDATOR:WP-1-Workspace-Safety-Parallel-Sessions-v1` session paperwork outside the governance refactor board
- OUTCOME: receipt-driven review reconciliation is now the sole writer for review-stage packet/runtime/task-board truth, live notification and lane-brief surfaces collapse unread history down to the current blocking route by default, and explicit `--history` access preserves crash-recovery residue for audits without polluting operator status

### 2026.04.08.3 / GOV-CHANGE-20260408-03

- STATUS: APPLIED
- SUMMARY: completed `RGF-152` by centralizing the live `phase-check` command string and widening command-drift coverage across the active role-facing docs
- CHANGE_TYPE: PHASE_CHECK_COMMAND_SOT_AND_ACTIVE_DOC_DRIFT_GUARD
- DRIVER_EVIDENCE:
  - `RGF-152`
  - operator follow-on after the 2026-04-08 governance refactor closeout
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/checks/phase-check-lib.mjs`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
  - `.GOV/roles_shared/tests/governance-command-contract.test.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/validator/checks/external-validator-brief.mjs`
  - `.GOV/roles/validator/tests/validator-next.test.mjs`
  - `.GOV/roles/validator/README.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
- OUTCOME: live helper output now builds the canonical `just phase-check ...` command from one shared source instead of repeating string literals across startup prompts and validator helpers, active command docs fail fast if they drift away from the justfile or reintroduce retired named phase wrappers, and the low-level leaf checks remain in-repo for now because they are still runtime dependencies of the composite boundary gate rather than archive-ready dead files

### 2026.04.08.4 / GOV-CHANGE-20260408-04

- STATUS: APPLIED
- SUMMARY: completed `RGF-153` by turning `phase-check` into an in-process composite runner, moving coder phase gating off subprocess hops where safe, and retiring the dead thin wrapper entrypoints
- CHANGE_TYPE: PHASE_RUNNER_IN_PROCESS_CONSOLIDATION_AND_WRAPPER_RETIREMENT
- DRIVER_EVIDENCE:
  - `RGF-153`
  - operator follow-on after the 2026-04-08 governance refactor closeout
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/checks/phase-check-lib.mjs`
  - `.GOV/roles_shared/checks/gate-check.mjs`
  - `.GOV/roles_shared/checks/wp-communication-health-check.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/coder/checks/pre-work.mjs`
  - `.GOV/roles/coder/checks/post-work.mjs`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles_shared/tests/cwd-agnostic-shared-checks.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs`
  - `.GOV/roles/validator/tests/validator-entrypoint-path-safety.test.mjs`
  - `.GOV/roles/validator/README.md`
  - `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: startup/handoff/verdict/closeout phases now converge through one artifact-producing runner instead of a shell-only fan-out, coder pre/post phase gates resolve the shared packet-order check in-process for faster debugging, and the obsolete `active-lane-brief` plus `integration-validator-context-brief` wrapper files were archived under `../../scripts_archive/` and removed from the live repo surface

### 2026.04.08.5 / GOV-CHANGE-20260408-05

- STATUS: APPLIED
- SUMMARY: completed `RGF-154` by rehoming the validator handoff and final-lane closeout leaf checks into the existing validator libraries and deleting the standalone script files
- CHANGE_TYPE: VALIDATOR_LEAF_ENTRYPOINT_REHOME_AND_RETIREMENT
- DRIVER_EVIDENCE:
  - `RGF-154`
  - operator follow-on after the 2026-04-08 governance refactor closeout
- SURFACES:
  - `justfile`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/checks/phase-check-lib.mjs`
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/tests/validator-next.test.mjs`
  - `.GOV/roles/validator/tests/validator-entrypoint-path-safety.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles_shared/tests/governance-command-contract.test.mjs`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles/validator/README.md`
  - `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: the `validator-handoff-check` and `integration-validator-closeout-check` commands still exist for compatibility, but they now run through existing validator library entrypoints, `phase-check` no longer depends on the retired standalone files, and two more repo-local validator check files were archived under `../../scripts_archive/` and removed from the live tree

### 2026.04.08.6 / GOV-CHANGE-20260408-06

- STATUS: APPLIED
- SUMMARY: completed `RGF-155` by rehoming validator packet completeness into the shared validator governance library and deleting the final standalone validator phase leaf
- CHANGE_TYPE: VALIDATOR_PACKET_HYGIENE_REHOME_AND_FINAL_PHASE_LEAF_RETIREMENT
- DRIVER_EVIDENCE:
  - `RGF-155`
  - operator follow-on after the 2026-04-08 governance refactor closeout
- SURFACES:
  - `justfile`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/checks/phase-check-lib.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/README.md`
  - `.GOV/roles_shared/records/COMPATIBILITY_SHIM_LEDGER.md`
  - `.GOV/roles_shared/records/GOVERNANCE_PREVENTION_LADDER.md`
  - `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
  - `.GOV/roles/validator/tests/validator-next.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles_shared/tests/governance-command-contract.test.mjs`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
- OUTCOME: packet-completeness enforcement is now part of the same validator governance library that already owns handoff hygiene, `phase-check` and closeout sync consume that logic in-process, the compatibility `just validator-packet-complete` surface remains available through the library entrypoint, and the final repo-local standalone validator phase-leaf file was archived under `../../scripts_archive/` and removed from the live tree

### 2026.04.08.7 / GOV-CHANGE-20260408-07

- STATUS: APPLIED
- SUMMARY: added a dedicated archive index so retired script/test files are documented by source path, archive location, former live purpose, and replacement surface
- CHANGE_TYPE: ARCHIVE_BOOKKEEPING_HARDENING
- DRIVER_EVIDENCE:
  - operator follow-on after `RGF-155`
- SURFACES:
  - `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`
- OUTCOME: the rationalization log now also carries a current-state archive inventory, so operators can answer "what was archived, where did it come from, and what replaced it?" without reconstructing that from terse log rows

### 2026.04.08.8 / GOV-CHANGE-20260408-08

- STATUS: APPLIED
- SUMMARY: completed `RGF-156` by cutting HANDOFF over to canonical `phase-check` commands, archiving the retired `post-work` / validator phase shim surfaces, and removing those shim recipes from the live justfile
- CHANGE_TYPE: HANDOFF_PHASE_SURFACE_CONSOLIDATION
- DRIVER_EVIDENCE:
  - `RGF-156`
  - operator follow-on after the 2026-04-08 governance refactor closeout
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/checks/phase-check-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/role-resume-utils.mjs`
  - `.GOV/roles/coder/checks/post-work.mjs`
  - `.GOV/roles/coder/README.md`
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/START_HERE.md`
  - `.GOV/roles_shared/docs/QUALITY_GATE.md`
  - `.GOV/roles_shared/docs/VALIDATOR_FILE_TOUCH_MAP.md`
  - `.GOV/roles_shared/scripts/hooks/pre-commit`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
  - `.GOV/roles/coder/tests/coder-command-surface.test.mjs`
  - `.GOV/roles/coder/tests/coder-entrypoint-path-safety.test.mjs`
  - `.GOV/roles/validator/tests/validator-command-surface.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles_shared/tests/governance-command-contract.test.mjs`
  - `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: the HANDOFF phase now has one authoritative shared runner for both coder-side closure and validator-side boundary proof, deterministic `--range` / `--rev` selectors flow through `phase-check HANDOFF ... CODER`, the old `post-work` file plus retired phase shim recipe definitions were archived under `../../scripts_archive/`, and the live justfile/docs/helper outputs no longer expose those shim surfaces

### 2026.04.08.9 / GOV-CHANGE-20260408-09

- STATUS: APPLIED
- SUMMARY: completed `RGF-157` by cutting STARTUP over to canonical `phase-check` commands, archiving the retired `pre-work` / `gate-check` surfaces, and removing those startup shim recipes/files from the live repo
- CHANGE_TYPE: STARTUP_PHASE_SURFACE_CONSOLIDATION
- DRIVER_EVIDENCE:
  - `RGF-157`
  - operator follow-on after the 2026-04-08 governance refactor closeout
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/checks/phase-check-lib.mjs`
  - `.GOV/roles/coder/checks/pre-work-check.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/coder/scripts/coder-next.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-prepare-and-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles_shared/checks/README.md`
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles_shared/tests/governance-command-contract.test.mjs`
  - `.GOV/roles/coder/checks/pre-work.mjs`
  - `.GOV/roles_shared/checks/gate-check.mjs`
  - `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: the STARTUP boundary now has one canonical public gate (`just phase-check STARTUP WP-{ID} CODER`), active governance surfaces no longer emit `just pre-work` / `just gate-check`, the retired startup script files and recipe definitions were archived under `../../scripts_archive/`, and future drift is mechanically blocked by the updated command-contract/alignment checks

### 2026.04.08.10 / GOV-CHANGE-20260408-10

- STATUS: APPLIED
- SUMMARY: completed `RGF-158` by letting `phase-check CLOSEOUT` optionally own the governed closeout truth sync, deferring final memory refresh until after sync, and rebasing active helper/docs surfaces onto the phase-owned closeout command
- CHANGE_TYPE: CLOSEOUT_PHASE_SYNC_INTEGRATION
- DRIVER_EVIDENCE:
  - `RGF-158`
  - operator follow-on after the 2026-04-08 governance refactor closeout
- SURFACES:
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/checks/phase-check-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/validator/tests/validator-command-surface.test.mjs`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: operators can now use `just phase-check CLOSEOUT ... --sync-mode ... --context ...` as the preferred phase-owned closeout command, the same artifact now carries closeout proof plus governed truth sync plus final memory refresh when sync is requested, and the remaining standalone `integration-validator-closeout-sync` command is demoted to a support/debug writer pending full retirement

### 2026.04.08.11 / GOV-CHANGE-20260408-11

- STATUS: APPLIED
- SUMMARY: completed `RGF-159` by retiring the standalone `integration-validator-closeout-sync` recipe, archiving its definition, and making `phase-check CLOSEOUT` the only live closeout command operators need to know
- CHANGE_TYPE: CLOSEOUT_RECIPE_RETIREMENT
- DRIVER_EVIDENCE:
  - `RGF-159`
  - operator follow-on after the 2026-04-08 governance refactor closeout
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/validator/tests/validator-command-surface.test.mjs`
  - `.GOV/roles_shared/tests/governance-command-contract.test.mjs`
  - `.GOV/docs_repo/GOVERNANCE_PHASE_CONSOLIDATION_LOG_2026-04-08.md`
  - `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: closeout mutation now runs only through `phase-check CLOSEOUT ... --sync-mode ... --context ...`, the phase runner performs its own repomem gate/context capture before invoking the governed writer directly, the retired recipe body is preserved under `../../scripts_archive/justfile/retired-governance-phase-shims-20260408.just`, and active command surfaces no longer expose a second closeout command

### 2026.04.09.1 / GOV-CHANGE-20260409-01

- STATUS: APPLIED
- SUMMARY: added trigger-aware and role-weighted governance memory recall so existing action entry points inject command-specific pitfalls and role habits before general findings
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260409-TRIGGER-AWARE-MEMORY-RECALL-INJECTION`
- SURFACES:
  - `.GOV/roles_shared/scripts/memory/memory-recall.mjs`
  - `.GOV/roles_shared/tests/memory-recall.test.mjs`
  - `.GOV/Audits/audits/AUDIT_20260409_TRIGGER_AWARE_MEMORY_RECALL_INJECTION.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `NONE`
- OUTCOME: `memory-recall` now uses stored trigger, script, and role metadata to surface repeated command failures and role-authored habits earlier, while keeping the existing public command surface stable inside the gov-kernel worktree

### 2026.04.09.2 / GOV-CHANGE-20260409-02

- STATUS: APPLIED
- SUMMARY: opened `RGF-162` to track ACP governed-role launch and steer reliability as a first-class state-machine hardening item instead of treating repeated attempts as model-quality drift
- CHANGE_TYPE: RECORDKEEPING_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-162`
  - `WP-1-Governance-Workflow-Mirror-v1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-162`
- OUTCOME: ACP launch/steer retries, broker drift recovery, self-settlement, and explicit busy/pending command states are now tracked as concrete governance implementation work that can proceed while Activation Manager continues refinement-heavy pre-launch work elsewhere

### 2026.04.09.3 / GOV-CHANGE-20260409-03

- STATUS: APPLIED
- SUMMARY: tightened the direct coder<->WP-validator contract so per-MT overlap review is the normal orchestrator-managed lane, while opening `RGF-163` for the remaining mechanical enforcement work
- CHANGE_TYPE: POLICY_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-163`
  - 2026-04-09 operator directive on coder/wp-validator communication and non-relay workflow
- SURFACES:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-163`
- OUTCOME: governance law now makes per-MT direct review and validator-owned steering explicit on orchestrator-managed lanes, clarifies deferred loop-back repair after validator disapproval, and reaffirms that the Orchestrator should only step in for governance workflow defects rather than relay ordinary coder/validator traffic

### 2026.04.09.3 / GOV-CHANGE-20260409-03

- STATUS: APPLIED
- SUMMARY: made governed memory injection visibly auditable and restored bounded startup prompt memory injection
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260409-VISIBLE-MEMORY-INJECTION-AND-STARTUP-BOUNDS`
- SURFACES:
  - `.GOV/roles_shared/scripts/memory/memory-recall.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/tests/memory-recall.test.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/Audits/audits/AUDIT_20260409_VISIBLE_MEMORY_INJECTION_AND_STARTUP_BOUNDS.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `NONE`
- OUTCOME: governed `memory-recall` now prints a compact `MEMORY_INJECTION_APPLIED` summary for operator visibility, and governed startup prompts once again carry a small bounded fail/context block instead of leaving the startup loader code disconnected from the active prompt builder

### 2026.04.09.4 / GOV-CHANGE-20260409-04

- STATUS: APPLIED
- SUMMARY: made the Memory Manager mechanical pre-pass conservative and report-first instead of letting automatic runs make destructive judgment calls
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260409-MEMORY-MANAGER-MECHANICAL-CONSERVATISM`
- SURFACES:
  - `.GOV/roles/memory_manager/scripts/launch-memory-manager.mjs`
  - `.GOV/roles/memory_manager/scripts/memory-manager-policy.mjs`
  - `.GOV/roles/memory_manager/tests/memory-manager-policy.test.mjs`
  - `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`
  - `.GOV/Audits/audits/AUDIT_20260409_MEMORY_MANAGER_MECHANICAL_CONSERVATISM.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `NONE`
- OUTCOME: automatic startup/closeout hygiene now performs soft decay and deterministic repair only, while stale, contradictory, and old low-value memories are surfaced as report-only candidates for the intelligent Memory Manager review instead of being auto-pruned or auto-consolidated

### 2026.04.09.5 / GOV-CHANGE-20260409-05

- STATUS: APPLIED
- SUMMARY: restored real governed receipt emission for packetless Memory Manager ACP lanes
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260409-MEMORY-MANAGER-PACKETLESS-RECEIPT-EMISSION`
- SURFACES:
  - `.GOV/roles/memory_manager/scripts/memory-manager-receipt.mjs`
  - `.GOV/roles/memory_manager/tests/memory-manager-receipt.test.mjs`
  - `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RECEIPT.schema.json`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communications-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `justfile`
  - `.GOV/Audits/audits/AUDIT_20260409_MEMORY_MANAGER_PACKETLESS_RECEIPT_EMISSION.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-160`
- OUTCOME: synthetic `WP-MEMORY-HYGIENE_<timestamp>` sessions now create packetless communication scaffolds, Memory Manager can emit `MEMORY_PROPOSAL` / `MEMORY_FLAG` / `MEMORY_RGF_CANDIDATE` receipts plus ORCHESTRATOR notifications through explicit just commands, and the governed startup/steering prompts now tell the role to use that surface instead of assuming an official packet-backed lane

### 2026.04.09.6 / GOV-CHANGE-20260409-06

- STATUS: APPLIED
- SUMMARY: opened `RGF-166` / `RGF-167` and implemented the first non-LLM relay watchdog slice for orchestrator-managed lanes
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-166`
  - `RGF-167`
  - 2026-04-09 operator directive on token-efficient autonomous relay watching, early stall capture, and removing screen babysitting from orchestrator-managed flow
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/lib/wp-relay-watchdog-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs`
  - `.GOV/roles/orchestrator/tests/wp-relay-watchdog-lib.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `justfile`
- FOLLOW_ON_ITEMS:
  - `RGF-166`
  - `RGF-167`
- OUTCOME: governance now has a public `just wp-relay-watchdog` command that consumes the existing receipt/notification/relay-escalation truth and safely re-steers only when the projected target is not already running; active runs are inspected with the existing stall scanner and reported conservatively as stalled rather than being killed by default, which establishes the mechanical watcher boundary for a later bounded repair ladder

### 2026.04.10.1 / GOV-CHANGE-20260410-01

- STATUS: APPLIED
- SUMMARY: started the `RGF-167` repair-ladder slice with bounded relay-cycle persistence and explicit repair signaling
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-167`
  - 2026-04-10 operator directive to commit the bounded auto-repair slice first and continue into repair handling
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/lib/wp-relay-watchdog-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles/orchestrator/tests/wp-relay-watchdog-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-167`
- OUTCOME: the watchdog now persists bounded relay-cycle state into WP runtime truth, surfaces relay-cycle budget in session-registry inspection, and emits deduped ORCHESTRATOR-targeted repair signals when a lane hits a true stalled-active-run or exhausted-relay-budget condition, without introducing destructive auto-cancel behavior by default

### 2026.04.10.2 / GOV-CHANGE-20260410-02

- STATUS: APPLIED
- SUMMARY: excluded packetless Memory Manager hygiene lanes from worktree concurrency gating and began command-family shell memory injection
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `procedural memory #776`
  - 2026-04-10 operator directive to remediate the `gov-check` blocker, then continue the two memory follow-up items
- SURFACES:
  - `.GOV/roles_shared/checks/worktree-concurrency-check.mjs`
  - `.GOV/roles_shared/tests/worktree-concurrency-check.test.mjs`
  - `.GOV/roles_shared/scripts/memory/memory-recall.mjs`
  - `.GOV/roles_shared/scripts/memory/shell-with-memory.mjs`
  - `.GOV/roles_shared/tests/memory-recall.test.mjs`
  - `.GOV/roles_shared/tests/shell-with-memory.test.mjs`
  - `justfile`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-160`
  - `RGF-168`
- OUTCOME: `gov-check` no longer treats synthetic `WP-MEMORY-HYGIENE_<ts>` Memory Manager lanes as product WPs that require coder/WP-validator worktrees, and governance now has an initial command-family shell wrapper that performs trigger-aware recall before ad hoc shell commands and can capture structured `shell-command` procedural memory keyed by command family

### 2026.04.10.3 / GOV-CHANGE-20260410-03

- STATUS: APPLIED
- SUMMARY: added a conservative default-off restart rung to the relay watchdog and documented the guardrail contract
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-167`
  - 2026-04-10 operator directive to proceed after evaluating restart safeguards for stalled governed lanes
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/lib/wp-relay-watchdog-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs`
  - `.GOV/roles/orchestrator/tests/wp-relay-watchdog-lib.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
- FOLLOW_ON_ITEMS:
  - `RGF-167`
- OUTCOME: the relay watchdog now supports an explicit `--allow-restart` repair rung that stays default-off and only cancels plus re-steers a governed lane when the projected target is one of `CODER` / `WP_VALIDATOR` / `INTEGRATION_VALIDATOR`, the lane is already classified as `REPORT_STALLED_ACTIVE_RUN`, the target session still claims `COMMAND_RUNNING`, the output file and session activity are both older than the configured freshness threshold, and every matching active run is already past `timeout_at`; otherwise the watchdog remains in report/escalate mode without destructive intervention

### 2026.04.10.4 / GOV-CHANGE-20260410-04

- STATUS: APPLIED
- SUMMARY: completed the Memory Manager ACP proof lane and hardened shell-command memory forwarding on the live `just` surface
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260410-MEMORY-MANAGER-COMPLETION-AND-COMMAND-MEMORY-FORWARDING`
  - `procedural memory #791`
  - 2026-04-10 follow-on after the first ACP Memory Manager proof and the operator-directed shell-memory injection work
- SURFACES:
  - `.GOV/Audits/audits/AUDIT_20260410_MEMORY_MANAGER_COMPLETION_AND_COMMAND_MEMORY_FORWARDING.md`
  - `.GOV/roles_shared/scripts/lib/node-argv-proxy.mjs`
  - `.GOV/roles_shared/tests/node-argv-proxy.test.mjs`
  - `.GOV/roles_shared/scripts/memory/governance-memory-cli.mjs`
  - `.GOV/roles_shared/scripts/memory/shell-with-memory.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/checks/wp-communications-check.mjs`
  - `.GOV/roles_shared/tests/wp-communications-check.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `justfile`
- FOLLOW_ON_ITEMS:
  - `RGF-160`
  - `RGF-168`
- OUTCOME: packetless Memory Manager ACP lanes now survive `gov-check`, completion is documented through governed `SESSION_COMPLETION` plus `repomem close`, and the `shell-with-memory` / `repomem` command family now forwards variadic flag text safely through PowerShell while routing structured shell-command memory through the canonical memory CLI

### 2026.04.11.1 / GOV-CHANGE-20260411-01

- STATUS: APPLIED
- SUMMARY: retroactively recorded the Workflow Dossier migration and ACP mechanical dossier trace as concrete governance workstreams
- CHANGE_TYPE: RECORDKEEPING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT_20260410_GOVERNANCE_WORKFLOW_MIRROR_ACTIVATION_MANAGER_SMOKETEST_REVIEW.md`
  - 2026-04-10/11 operator directive on live run dossiers, ACP printouts, closeout drift control, and rubric-at-closeout only
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-171`
- OUTCOME: the board now has stable IDs for the already-applied Workflow Dossier and ACP dossier-trace work (`RGF-169` / `RGF-170`), so later audits, memory extraction, and follow-on items can cite concrete workstreams instead of only operator chat or broad audit filenames

### 2026.04.11.2 / GOV-CHANGE-20260411-02

- STATUS: APPLIED
- SUMMARY: marked `RGF-162` in progress after the headless ACP launch slice and opened grouped follow-ons for downtime metrics, explicit result states, single-attempt recovery, and bridge-era compatibility cleanup
- CHANGE_TYPE: RECORDKEEPING_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-162`
  - `AUDIT_20260410_GOVERNANCE_WORKFLOW_MIRROR_ACTIVATION_MANAGER_SMOKETEST_REVIEW.md`
  - 2026-04-11 operator directive on ACP launch reliability, launch polish, and downtime reduction
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-171`
  - `RGF-172`
  - `RGF-173`
  - `RGF-174`
- OUTCOME: governance now explicitly records that `RGF-162` has started landing through the headless/direct `AUTO` ACP slice and failure-visible dossier logging, while the remaining ACP reliability and downtime work is grouped into concrete next-wave items instead of lingering as loose notes

### 2026.04.11.3 / GOV-CHANGE-20260411-03

- STATUS: APPLIED
- SUMMARY: removed the historical duplicate `RGF-163` board collision by re-keying the visible-memory/startup-restore workstream to `RGF-175`
- CHANGE_TYPE: RECORDKEEPING_HARDENING
- DRIVER_EVIDENCE:
  - duplicate `RGF-163` collision found in `REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `AUDIT-20260409-VISIBLE-MEMORY-INJECTION-AND-STARTUP-BOUNDS`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `NONE`
- OUTCOME: `RGF-163` now refers only to the per-MT validator steering workstream, while the already-completed visible memory injection and bounded startup restore work has a unique stable board ID (`RGF-175`) for future audit and memory references

### 2026.04.11.4 / GOV-CHANGE-20260411-04

- STATUS: APPLIED
- SUMMARY: started `RGF-172` by introducing explicit ACP `outcome_state` reporting and idempotent `START_SESSION` handling for already-ready governed sessions
- CHANGE_TYPE: SESSION_CONTROL_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-172`
  - 2026-04-11 operator directive on repeated ACP launch attempts, launch polish, and reducing workflow downtime
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-172`
  - `RGF-173`
- OUTCOME: governed session-control results now carry a machine-readable `outcome_state`, `session-start` returns `ALREADY_READY` instead of failing when a steerable thread is already registered, broker concurrency is surfaced as `BUSY_ACTIVE_RUN`, and the Workflow Dossier/operator surface no longer collapses those steady states into generic opaque failures

### 2026.04.11.5 / GOV-CHANGE-20260411-05

- STATUS: APPLIED
- SUMMARY: started `RGF-173` by making the ACP broker recover stale same-session active runs in the same request path before returning `BUSY_ACTIVE_RUN`
- CHANGE_TYPE: SESSION_CONTROL_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-173`
  - 2026-04-11 operator directive on repeated ACP launch attempts, stale busy states, and removing visible launch slop
- SURFACES:
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-self-settle-lib.mjs`
  - `.GOV/roles_shared/tests/session-control-self-settle-lib.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-173`
  - `RGF-174`
- OUTCOME: if a governed lane appears busy only because the broker still holds a same-session run whose child process already died, whose timeout already expired, or whose terminal result row already exists, the broker now repairs or prunes that stale run inside the same launch/steer attempt and only returns `BUSY_ACTIVE_RUN` when a genuinely live competing run remains

### 2026.04.11.6 / GOV-CHANGE-20260411-06

- STATUS: APPLIED
- SUMMARY: completed `RGF-174` by retiring bridge-era packet law from the ordinary path and bounding the remaining launch queue/runtime surfaces to explicit compatibility usage
- CHANGE_TYPE: SESSION_POLICY_MIGRATION
- DRIVER_EVIDENCE:
  - `RGF-174`
  - 2026-04-11 operator directive after removing the local VS Code bridge and asking for lingering plugin-first residue to be cleaned up
- SURFACES:
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles_shared/checks/session-launch-runtime-check.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs`
  - `.GOV/roles/coder/checks/pre-work-check.mjs`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles_shared/docs/ROLE_WORKTREES.md`
  - `.GOV/roles/orchestrator/README.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-172`
  - `RGF-173`
- OUTCOME: new packets/stubs now describe ACP control ledgers as the ordinary launch/steer path, legacy bridge queue fields are no longer stamped into fresh packet law, pre-work accepts both the new ACP-first shape and older bridge-era packets, and the remaining `VSCODE_PLUGIN` queue/runtime artifacts are explicitly framed as compatibility-only instead of the normal governed launch path

### 2026.04.11.7 / GOV-CHANGE-20260411-07

- STATUS: APPLIED
- SUMMARY: completed `RGF-171` by adding a live Workflow Dossier idle ledger that appends mechanical latency and drift metrics during dossier sync
- CHANGE_TYPE: DOSSIER_OBSERVABILITY
- DRIVER_EVIDENCE:
  - `RGF-171`
  - 2026-04-11 operator directive on reducing workflow downtime, closeout drift, and token cost aggressively where possible
- SURFACES:
  - `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs`
  - `.GOV/roles_shared/tests/wp-timeline-lib.test.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier-lib.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
  - `.GOV/templates/WORKFLOW_DOSSIER_TEMPLATE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/operator/docs_local/Handshake_Role_Startup_Prompts.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-172`
  - `RGF-173`
- OUTCOME: `just workflow-dossier-sync WP-{ID}` now appends both the ACP/runtime execution summary and a separate `LIVE_IDLE_LEDGER` line with request-to-response latency, validator-pass-to-coder latency, current/max idle gaps, and compact drift markers so stall diagnosis is mechanical during the run instead of reconstructed from closeout memory

### 2026.04.11.8 / GOV-CHANGE-20260411-08

- STATUS: APPLIED
- SUMMARY: advanced the next `RGF-172` / `RGF-173` slice by reconciling recoverable broker state before restart refusal and by letting `START_SESSION` absorb the common busy-while-becoming-ready race
- CHANGE_TYPE: SESSION_CONTROL_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-172`
  - `RGF-173`
  - 2026-04-11 operator directive to continue the remaining ACP reliability work after the bridge cleanup and dossier idle-metrics slices
- SURFACES:
  - `.GOV/roles_shared/scripts/session/handshake-acp-client.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles_shared/tests/handshake-acp-client.test.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-control-self-settle-lib.test.mjs`
  - `.GOV/roles_shared/checks/session-control-runtime-check.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-172`
  - `RGF-173`
- OUTCOME: the ACP client now prunes/self-settles recoverable broker-state residue before deciding that a mismatched or unreachable broker still has blocking active runs, and the `START_SESSION` wrapper now waits briefly for the session to become READY before failing `BUSY_ACTIVE_RUN` / `REQUIRES_RECOVERY`, which removes one common double-launch race without pretending that genuinely live competing runs are safe

### 2026.04.11.9 / GOV-CHANGE-20260411-09

- STATUS: APPLIED
- SUMMARY: opened `RGF-176` to require a real post-refactor ACP launch proof on a non-terminal WP and to preserve the concrete watchpoints for that validation
- CHANGE_TYPE: GOVERNANCE_PLANNING
- DRIVER_EVIDENCE:
  - `RGF-176`
  - 2026-04-11 live `just launch-activation-manager-session WP-1-Governance-Workflow-Mirror-v1 AUTO PRIMARY` attempt was blocked before ACP by terminal task-board status `OUTDATED_ONLY`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-176`
- OUTCOME: the board now explicitly tracks the remaining practical validation gap: a real `AUTO` ACP launch on a non-terminal WP must still be exercised and checked for one-attempt convergence, no focus-stealing terminal launch, no duplicate request/result rows, truthful `outcome_state`, bounded token/ledger churn, clean Workflow Dossier ACP traces, and no stale broker/session residue after settlement

### 2026.04.12.1 / GOV-CHANGE-20260412-01

- STATUS: APPLIED
- SUMMARY: opened RGF-189 through RGF-193 for governance workflow restructure — mechanical orchestrator, Rubrik role, Integration Validator expansion, orchestrator protocol update, closeout auto-repair
- CHANGE_TYPE: GOVERNANCE_PLANNING
- DRIVER_EVIDENCE:
  - `DOSSIER_20260412_GOVERNANCE_WORKFLOW_MIRROR_WORKFLOW_DOSSIER.md` — 329min wall clock, 4.4min active, 256M tokens_in for 1 MT; 85% cost from mechanical work routed through ACP sessions
  - Operator diagnosis: workflow-state drift is the dominant cost, not document drift; closeout truth/artifact drift is second
  - Recurring product/repo governance confusion causing coder scope spill (dossier line 1547, feedback memories)
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-189` — Orchestrator Mechanical Governance and No-Approval Boundary
  - `RGF-190` — Rubrik Role Protocol (per-MT boundary enforcement, product/repo containment)
  - `RGF-191` — Integration Validator Whole-WP Judgment and Merge Authority
  - `RGF-192` — Orchestrator Protocol Update for Role-Split Workflow
  - `RGF-193` — Closeout Auto-Repair Script
- OUTCOME: the governance task board now tracks five new items for restructuring the governed workflow to separate mechanical governance (direct script execution by orchestrator) from AI-mediated judgment (Rubrik per-MT, Integration Validator whole-WP); classic VALIDATOR role preserved for manual relay workflow

### 2026.04.12.2 / GOV-CHANGE-20260412-02

- STATUS: APPLIED
- SUMMARY: implemented RGF-189 through RGF-193 — created Rubrik role protocol, Integration Validator protocol, updated Orchestrator Protocol with mechanical governance + no-approval boundary + role-split workflow, added RUBRIK to session registry schema + session policy, created closeout-repair script, added justfile recipes for Rubrik sessions and closeout-repair
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-189` through `RGF-193`
  - `DOSSIER_20260412_GOVERNANCE_WORKFLOW_MIRROR_WORKFLOW_DOSSIER.md` (cost evidence)
  - Operator direction: Rubrik for per-MT boundary enforcement, Integration Validator for whole-WP judgment, orchestrator runs mechanical checks directly
- SURFACES:
  - `.GOV/roles/rubrik/RUBRIK_PROTOCOL.md` (new)
  - `.GOV/roles/validator/INTEGRATION_VALIDATOR_PROTOCOL.md` (new)
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (updated: mechanical governance principle, no-approval boundary, role-split workflow, Rubrik references)
  - `.GOV/roles_shared/schemas/ROLE_SESSION_REGISTRY.schema.json` (added RUBRIK to role enum)
  - `.GOV/roles_shared/scripts/session/session-policy.mjs` (added RUBRIK to SESSION_ROLES, model profiles, token budgets, terminal title, command mappings)
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs` (added RUBRIK to usage)
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs` (added RUBRIK to usage)
  - `.GOV/roles/orchestrator/scripts/session-control-cancel.mjs` (added RUBRIK to role list + usage)
  - `.GOV/roles/orchestrator/scripts/role-session-worktree-add.mjs` (added RUBRIK to usage)
  - `.GOV/roles/orchestrator/scripts/closeout-repair.mjs` (new)
  - `.GOV/roles/README.md` (added Rubrik and Integration Validator links)
  - `justfile` (added launch/start/steer/cancel/close-rubrik-session, closeout-repair recipes)
- FOLLOW_ON_ITEMS:
  - `RGF-189` through `RGF-193` (status update to DONE pending first governed run validation)
- OUTCOME: governance framework now implements the role split: Rubrik handles per-MT boundary enforcement and code review, Integration Validator handles whole-WP spec judgment with fresh context, Orchestrator runs mechanical governance directly; classic Validator preserved for manual relay; `just gov-check` passes

### 2026.04.12.3 / GOV-CHANGE-20260412-03

- STATUS: APPLIED
- SUMMARY: corrected RGF-190-193 implementation — removed RUBRIK as role/folder (evaluation criteria inlined into WP_VALIDATOR_PROTOCOL), fixed Integration Validator direct-coder contradiction, added FAIL remediation flow, added sync-gov-to-main as explicit IntVal duty, added orchestrator role definition block, added bounded loop reference (RGF-100), added closeout-repair failure recovery path, created classic_orchestrator role for manual relay workflow
- CHANGE_TYPE: GOVERNANCE_CORRECTION
- DRIVER_EVIDENCE:
  - Operator correction: RUBRIK is not a role, not a document, not anything — evaluation criteria belong inside WP_VALIDATOR_PROTOCOL directly
  - Operator correction: orchestrator does not create refinements, worktrees, MTs (Activation Manager does)
  - Operator correction: WP Validator does not actively steer coder (saves tokens), mechanical stall detection instead
  - ORCHESTRATOR_PROTOCOL line 752 contradicted INTEGRATION_VALIDATOR_PROTOCOL line 136 (direct coder communication)
  - Manual relay workflow mixed into orchestrator protocol causing confusion
- SURFACES:
  - `.GOV/roles/rubrik/` (deleted)
  - `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md` (rewritten — full standalone protocol with inline evaluation criteria)
  - `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md` (updated — FAIL remediation flow, sync-gov-to-main duty, removed rubrik refs)
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (updated — role definition block, removed rubrik refs, fixed line 752 contradiction, bounded loop, closeout recovery)
  - `.GOV/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md` (new — manual relay workflow role identity)
  - `.GOV/roles/README.md` (updated — removed rubrik, added classic_orchestrator)
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` (corrected RGF-190/192 descriptions)
- OUTCOME: RUBRIK concept fully removed from governance; WP Validator is standalone per-MT reviewer with inline evaluation criteria; Integration Validator FAIL flow routes remediation through Orchestrator; orchestrator role narrowed to launch+watch+steer with mechanical stall detection; classic_orchestrator role created for manual relay workflow separation

### 2026.04.12.4 / GOV-CHANGE-20260412-04

- STATUS: APPLIED
- SUMMARY: post-restructure sweep — fixed critical Integration Validator startup prompt (was telling IntVal to communicate directly with coder via structured review lane, contradicting protocol), cleaned 3 stale KUBRIK/RUBRIK memory entries from governance DB, fixed misleading doc heading, opened RGF-194 for check script consolidation (49→~15-20 bundled groups)
- CHANGE_TYPE: GOVERNANCE_CORRECTION
- DRIVER_EVIDENCE:
  - session-control-lib.mjs line 898: startup prompt injected into IntVal sessions said "DIRECT COMMUNICATION (MANDATORY)" which contradicted INTEGRATION_VALIDATOR_PROTOCOL.md and orchestrator protocol line 765
  - governance memory DB entries 1555-1557 contained stale KUBRIK/RUBRIK references from the transition period
  - GOVERNED_WORKFLOW_EXAMPLES.md line 93 heading said "directly" when communication is packet-mediated
  - full audit of roles/ and roles_shared/ revealed 49 standalone check scripts as consolidation opportunity
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs` (fixed IntVal startup prompt — replaced DIRECT COMMUNICATION with VERDICT COMMUNICATION routing through orchestrator)
  - governance memory DB (deleted entries 1555, 1556, 1557)
  - `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md` (fixed heading)
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` (added RGF-194)
- OUTCOME: Integration Validator startup prompt now correctly instructs the model to route FAIL remediation through Orchestrator instead of communicating directly with coder; governance memory DB is clean of stale RUBRIK/KUBRIK entries; RGF-194 opened for future check consolidation

### 2026.04.18.1 / GOV-CHANGE-20260418-01

- STATUS: APPLIED
- SUMMARY: opened the current repo-governance hardening queue, reconciled board truth for bundled governance checks, and recorded the next implementation sequence around workflow truth, ACP resilience, and degraded-mode parity
- CHANGE_TYPE: GOVERNANCE_PLANNING
- DRIVER_EVIDENCE:
  - Operator directive on 2026-04-18 to convert the new repo-governance research into concrete taskboard items and start the first implementation set
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/Repo_Governance_Failure_Taxonomy.md`
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/Workflow_State_Packet_Truth_and_Range_Drift.md`
  - `.GOV/reference/research_and_papers/Multi_Model_Architecture/ACP_Broker_and_Session_Control.md`
  - `.GOV/Audits/smoketest/DOSSIER_20260412_GOVERNANCE_WORKFLOW_MIRROR_WORKFLOW_DOSSIER.md`
  - `.GOV/Audits/smoketest/DOSSIER_20260413_CALENDAR_STORAGE_WORKFLOW_DOSSIER.md`
  - `.GOV/Audits/smoketest/DOSSIER_20260413_PROJECT_AGNOSTIC_WORKFLOW_STATE_REGISTRY_WORKFLOW_DOSSIER.md`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-198`
  - `RGF-199`
  - `RGF-200`
  - `RGF-201`
  - `RGF-202`
  - `RGF-203`
- OUTCOME: the board now records the active repo-governance hardening queue, marks bundled governance checks (`RGF-194`) as already live, and sequences the next work around canonical workflow truth, ACP health, retry suppression, and manual-relay parity

### 2026.04.18.2 / GOV-CHANGE-20260418-02

- STATUS: APPLIED
- SUMMARY: implemented the first workflow-truth hardening slice by classifying packet/runtime drift by owning surface and surfacing repair order in `orchestrator-next`
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-198`
  - workflow truth drift research: `.GOV/reference/research_and_papers/Multi_Model_Architecture/Workflow_State_Packet_Truth_and_Range_Drift.md`
  - failure taxonomy: `.GOV/reference/research_and_papers/Multi_Model_Architecture/Repo_Governance_Failure_Taxonomy.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-199`
  - `RGF-200`
- OUTCOME: status-sync drift no longer presents as anonymous mismatch text only; the runtime evaluation now reports which truth surface owns each mismatch and the order repairs should follow before further delegation resumes

### 2026.04.18.3 / GOV-CHANGE-20260418-03

- STATUS: APPLIED
- SUMMARY: froze explicit committed handoff range and authoritative latest-review identity into runtime projection during WP communication reconciliation
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-199`
  - failure taxonomy and range-drift research around mutable `base..head` reconstruction during validator/closeout work
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-communications-lib.test.mjs`
  - `.GOV/roles_shared/tests/ensure-wp-communications.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-200`
- OUTCOME: runtime status now carries durable review anchor fields for the latest authoritative review receipt and any explicit committed handoff range, which reduces later dependence on mutable packet prose when validator and closeout surfaces need the same handoff truth

### 2026.04.18.4 / GOV-CHANGE-20260418-04

- STATUS: APPLIED
- SUMMARY: persisted canonical route-anchor truth into runtime status and reused it across route selection, notification visibility, boundary checks, and manual-relay fallback
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-200`
  - ACP/session control research: `.GOV/reference/research_and_papers/Multi_Model_Architecture/ACP_Broker_and_Session_Control.md`
  - workflow truth drift research: `.GOV/reference/research_and_papers/Multi_Model_Architecture/Workflow_State_Packet_Truth_and_Range_Drift.md`
  - workflow-mirror and calendar-storage smoketest dossiers documenting route drift and stalled wakeups
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles/orchestrator/scripts/lib/manual-relay-envelope-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/ensure-wp-communications.test.mjs`
  - `.GOV/roles_shared/tests/wp-communications-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/manual-relay-envelope-lib.test.mjs`
  - `.GOV/roles_shared/fixtures/wp-communication-health/09-verdict-waiting-for-final-review.json`
- FOLLOW_ON_ITEMS:
  - `RGF-201`
  - `RGF-202`
- OUTCOME: runtime status now stores `route_anchor_*` fields so reconciliation, boundary checks, and manual relay all converge on the same correlation and target; blocked review queues keep the anchored work item even if `open_review_items` reorder; verdict progression retains the authoritative final-review correlation after a completed review; and the waiting-for-final-review fixture now matches the deliberate VERDICT preflight rule that the lane can be mechanically healthy while still waiting for the Integration Validator exchange

### 2026.04.18.5 / GOV-CHANGE-20260418-05

- STATUS: APPLIED
- SUMMARY: added durable ACP session health projection, push-alert emission, and operator stop surfaces for degraded or failed governed sessions
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-201`
  - ACP/session control research: `.GOV/reference/research_and_papers/Multi_Model_Architecture/ACP_Broker_and_Session_Control.md`
  - failure taxonomy research: `.GOV/reference/research_and_papers/Multi_Model_Architecture/Repo_Governance_Failure_Taxonomy.md`
  - workflow-mirror, project-agnostic registry, and distillation smoketest dossiers documenting repeated stalls, lock-ups, and late session-health discovery
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-health-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-lane-health.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles_shared/tests/session-health-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-registry-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-202`
  - `RGF-203`
- OUTCOME: relay watchdog now computes governed-session health from runtime state, heartbeat age, active-run timeout, and command-output freshness; registry sessions persist `health_state`, `health_reason_code`, `health_summary`, and update time; degraded/failed sessions emit deduped `ACP_HEALTH_ALERT` notifications to `ORCHESTRATOR`; `orchestrator-next`, `session-registry-status`, and `wp-lane-health` surface the same health truth directly; and session reopen no longer rewrites previously approved launch-selection authority fields while this health state is being maintained across recovery loops

### 2026.04.18.6 / GOV-CHANGE-20260418-06

- STATUS: APPLIED
- SUMMARY: added same-failure rewake suppression so identical stale-route failures stop consuming repeated automatic governed wakes
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-202`
  - failure taxonomy research: `.GOV/reference/research_and_papers/Multi_Model_Architecture/Repo_Governance_Failure_Taxonomy.md`
  - ACP/session control research: `.GOV/reference/research_and_papers/Multi_Model_Architecture/ACP_Broker_and_Session_Control.md`
  - smoketest dossiers documenting repeated stalls, lock-ups, and re-wake loops on unchanged session/control-plane conditions
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/lib/wp-relay-watchdog-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles/orchestrator/tests/wp-relay-watchdog-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-203`
- OUTCOME: relay watchdog now derives a typed failure fingerprint for stale-route and stalled-run control-plane failures, stores same-failure rewake counters in runtime status, suppresses duplicate automatic `STEER` attempts when the identical failure state repeats without evidence movement, emits `RELAY_WATCHDOG_REPAIR` notifications for the suppressed state, and makes `orchestrator-next` stop on that repair notification instead of recommending another blind re-wake on unchanged conditions

### 2026.04.18.7 / GOV-CHANGE-20260418-07

- STATUS: APPLIED
- SUMMARY: normalized manual relay onto the same anchored route and packet/runtime drift truth used by the governed ACP lane
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-203`
  - workflow truth drift research: `.GOV/reference/research_and_papers/Multi_Model_Architecture/Workflow_State_Packet_Truth_and_Range_Drift.md`
  - harness comparison research: `.GOV/reference/research_and_papers/Multi_Model_Architecture/_work/Repo_Governance_Harness_Comparison_WORKING.md`
  - capability matrix and kernel-to-swarm gap research calling for one state model across autonomous and manual relay modes
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/lib/manual-relay-envelope-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/manual-relay-next.mjs`
  - `.GOV/roles/orchestrator/scripts/manual-relay-dispatch.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles/orchestrator/tests/manual-relay-envelope-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/manual-relay-next.test.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
- FOLLOW_ON_ITEMS:
  - none in the current repo-governance hardening queue
- OUTCOME: manual relay now prefers the runtime-anchored notification correlation when multiple stale candidate notifications exist, reuses the shared active-notification projection to hide non-live residue for the target role, surfaces hidden-notification counts to the classic operator view, and hard-stops manual-relay-next/manual-relay-dispatch when packet/runtime closeout truth drifts so degraded mode no longer bypasses the same workflow-state spine that governs ACP lanes

### 2026.04.19.1 / GOV-CHANGE-20260419-01

- STATUS: APPLIED
- SUMMARY: recorded the next repo-governance hardening tranche as six concrete governance-board items centered on canonical state, typed governed actions, queued ingress, runtime escalation, telemetry split, and closeout dependency collapse
- CHANGE_TYPE: GOVERNANCE_PLANNING
- DRIVER_EVIDENCE:
  - research synthesis across `Workflow_State_Packet_Truth_and_Range_Drift.md`, `_work/Repo_Governance_Harness_Comparison_WORKING.md`, `Repo_Governance_Failure_Taxonomy.md`, `_work/Typed_Runtime_Resume_Approval_WORKING.md`, `_work/Harness_Adoption_Extraction_WORKING.md`, `Harness_Lessons_Learned.md`, `Technical_Implementation_Research.md`, `ACP_Broker_and_Session_Control.md`, and `Validator_Routing_Gates_and_Closeout_Repair.md`
  - `RGF-204`
  - `RGF-205`
  - `RGF-206`
  - `RGF-207`
  - `RGF-208`
  - `RGF-209`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-204`
  - `RGF-205`
- OUTCOME: the governance board now carries the next implementation-ready hardening tranche instead of leaving the research synthesis as narrative only, with `RGF-204` designated as the first launchable slice and the remaining items sequenced behind that canonical-state cutover

### 2026.04.19.2 / GOV-CHANGE-20260419-02

- STATUS: APPLIED
- SUMMARY: landed the first `RGF-204` slice by introducing canonical runtime `execution_state` authority, checkpoint lineage, and restore helpers while keeping legacy flat runtime fields as compatibility projections
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-204`
  - research synthesis across `Workflow_State_Packet_Truth_and_Range_Drift.md`, `_work/Repo_Governance_Harness_Comparison_WORKING.md`, `Repo_Governance_Failure_Taxonomy.md`, and `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-execution-state-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/templates/WP_RUNTIME_STATUS_TEMPLATE.json`
  - `.GOV/roles_shared/tests/wp-execution-state-lib.test.mjs`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-204`
  - `RGF-205`
- OUTCOME: runtime truth now persists one canonical `execution_state.authority` block plus durable checkpoint lineage, the main packet/review projection writers stamp that canonical block on every sync, restore-to-checkpoint logic exists mechanically, and the existing flat runtime fields are now compatibility mirrors rather than the only durable authority surface

### 2026.04.19.3 / GOV-CHANGE-20260419-03

- STATUS: APPLIED
- SUMMARY: landed the second `RGF-204` slice by making the main read-side governance surfaces hydrate runtime truth from canonical `execution_state.authority` before evaluating route, closeout, or operator status
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-204`
  - research synthesis across `Workflow_State_Packet_Truth_and_Range_Drift.md`, `_work/Repo_Governance_Harness_Comparison_WORKING.md`, `Repo_Governance_Failure_Taxonomy.md`, and `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-execution-state-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/merge-progression-truth-lib.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-autonomous-monitor.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles_shared/tests/wp-execution-state-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/merge-progression-truth-lib.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-204`
  - `RGF-205`
- OUTCOME: the runtime read path no longer depends on flat-field truth reconstruction alone; health/routing logic, drift detection, merge-progress validation, resume tooling, autonomous monitoring, operator status, and workflow-dossier sync now materialize the runtime view from canonical `execution_state.authority` with flat fields treated as compatibility mirrors when the canonical block is present

### 2026.04.19.4 / GOV-CHANGE-20260419-04

- STATUS: APPLIED
- SUMMARY: hardened orchestrator-facing path discipline so governance/operator surfaces stop emitting host-specific absolute paths and the protocol now states that relative paths are mandatory on operator-facing outputs
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-204`
  - `CX-109`
- SURFACES:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/scripts/lib/role-resume-utils.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles_shared/docs/TOOLING_GUARDRAILS.md`
  - `.GOV/roles_shared/tests/role-resume-utils.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-204`
- OUTCOME: stale-worktree diagnostics and board/worktree monitor summaries now prefer repo-relative or workspace-relative paths, and the orchestrator law explicitly forbids absolute-path leakage on operator-facing governance surfaces while still allowing internal absolute-path resolution inside scripts

### 2026.04.19.5 / GOV-CHANGE-20260419-05

- STATUS: APPLIED
- SUMMARY: pushed the next `RGF-204` cutover into shared task-board and dossier status readers so canonical `execution_state` truth now drives session governance, active-lane briefs, and workflow-dossier sync instead of those surfaces independently reconstructing status from mixed packet/board/runtime mirrors
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-204`
  - research synthesis across `Workflow_State_Packet_Truth_and_Range_Drift.md`, `_work/Repo_Governance_Harness_Comparison_WORKING.md`, `Repo_Governance_Failure_Taxonomy.md`, and `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-execution-state-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-authority-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-governance-state-lib.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
  - `.GOV/roles_shared/tests/wp-execution-state-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-governance-state-lib.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-204`
  - `RGF-205`
- OUTCOME: board- and dossier-facing status summaries now share one canonical runtime projection helper, flat runtime mirrors no longer outrank packet/task-board artifacts unless an explicit `execution_state.authority` block exists, and terminal `DONE_*` runtime task-board states now resolve consistently across projection readers instead of silently appearing active

### 2026.04.19.6 / GOV-CHANGE-20260419-06

- STATUS: APPLIED
- SUMMARY: extended the `RGF-204` cutover into task-board mutation, closeout packet publication, and audit skeleton reporting so those publication surfaces now consume the shared canonical execution publication view instead of each deriving status independently from packet and board text
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-204`
  - research synthesis across `Workflow_State_Packet_Truth_and_Range_Drift.md`, `_work/Repo_Governance_Harness_Comparison_WORKING.md`, `Repo_Governance_Failure_Taxonomy.md`, and `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-execution-state-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/task-board-set.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-closeout-format.mjs`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
  - `.GOV/roles/orchestrator/tests/task-board-set.test.mjs`
  - `.GOV/roles_shared/tests/wp-closeout-format.test.mjs`
  - `.GOV/roles_shared/tests/generate-post-run-audit-skeleton.test.mjs`
  - `.GOV/roles_shared/tests/wp-execution-state-lib.test.mjs`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-204`
  - `RGF-205`
- OUTCOME: canonical-mode task-board writes now validate against `execution_state.authority` and only refresh runtime compatibility mirrors instead of re-deriving runtime from packet text, closeout formatting syncs updated packet publication back into runtime truth, and audit skeletons now report packet/task-board publication status from the shared canonical publication view when explicit execution authority exists

### 2026.04.19.7 / GOV-CHANGE-20260419-07

- STATUS: APPLIED
- SUMMARY: pushed the `RGF-204` cutover into integration-validator closeout/context helpers so final-lane readiness and terminal non-pass requirements now honor the declared runtime publication view before trusting stale packet or task-board artifacts
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-204`
  - research synthesis across `Workflow_State_Packet_Truth_and_Range_Drift.md`, `_work/Repo_Governance_Harness_Comparison_WORKING.md`, `Repo_Governance_Failure_Taxonomy.md`, and `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-204`
  - `RGF-209`
- OUTCOME: integration-validator closeout requirements and context briefs now load the declared runtime status file, prefer canonical `execution_state.authority` packet/task-board publication truth when it exists, and treat packet text or board artifacts as fallback only when canonical runtime authority is absent, reducing final-lane resume and closeout drift caused by stale packet archaeology

### 2026.04.19.8 / GOV-CHANGE-20260419-08

- STATUS: APPLIED
- SUMMARY: extended the `RGF-204` cutover into `phase-check CLOSEOUT` next-command generation so closeout sync suggestions no longer treat a stale PASS packet as terminal when canonical containment truth still says `MERGE_PENDING`
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-204`
  - research synthesis across `Workflow_State_Packet_Truth_and_Range_Drift.md`, `_work/Repo_Governance_Harness_Comparison_WORKING.md`, `Repo_Governance_Failure_Taxonomy.md`, and `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-204`
  - `RGF-209`
- OUTCOME: phase-check closeout suggestions now read the declared runtime publication view, infer the effective closeout verdict from canonical packet/task-board authority, and only skip directly to final PASS flow when containment is actually `CONTAINED_IN_MAIN` or `NOT_REQUIRED`, eliminating one more stale-packet shortcut in the closeout path

### 2026.04.19.9 / GOV-CHANGE-20260419-09

- STATUS: APPLIED
- SUMMARY: refactored `closeout-repair` so its diagnosis step now classifies failures from direct packet/runtime/helper truth instead of scraping `phase-check CLOSEOUT` output text, while keeping phase-check as the outer verifier
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-204`
  - `RGF-209`
  - research synthesis across `Workflow_State_Packet_Truth_and_Range_Drift.md`, `Validator_Routing_Gates_and_Closeout_Repair.md`, `_work/Repo_Governance_Harness_Comparison_WORKING.md`, and `Repo_Governance_Failure_Taxonomy.md`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/closeout-repair.mjs`
  - `.GOV/roles/orchestrator/tests/closeout-repair.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-204`
  - `RGF-209`
- OUTCOME: closeout-repair now inventories baseline drift, declared patch-artifact drift, clause/report drift, missing verdicts, packet completeness residue, communication-health failures, and integration-validator closeout blockers from direct helper state; it auto-repairs only the narrow mechanical set (baseline SHA sync and declared patch regeneration), uses packet-declared artifact paths instead of hardcoded patch locations, and leaves remaining closeout issues as explicit manual classes instead of guessing from phase-check transcript text

### 2026.04.19.10 / GOV-CHANGE-20260419-10

- STATUS: APPLIED
- SUMMARY: closed the remaining `RGF-204` residue by centralizing closeout mode inference and canonical publication ownership in the shared execution-state library, then rewiring final-lane callers to consume that shared truth
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-204`
  - research synthesis across `Workflow_State_Packet_Truth_and_Range_Drift.md`, `_work/Repo_Governance_Harness_Comparison_WORKING.md`, `Repo_Governance_Failure_Taxonomy.md`, and `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-execution-state-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles_shared/tests/wp-execution-state-lib.test.mjs`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-205`
  - `RGF-209`
- OUTCOME: canonical `execution_state.authority` now owns the remaining closeout/publication rule tables as well as packet-vs-runtime publication drift ownership, `phase-check` / `orchestrator-next` / `integration-validator-closeout-sync` consume the same shared closeout mode definitions, and `RGF-204` is complete because packet/task-board publication no longer depends on scattered helper-local closeout mappings in the terminal path

### 2026.04.20.1 / GOV-CHANGE-20260420-01

- STATUS: APPLIED
- SUMMARY: started `RGF-205` by introducing a registry-backed governed-action envelope into the session-control lane and projecting a bounded runtime action history for governed sessions
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-205`
  - typed resume/approval research: `_work/Typed_Runtime_Resume_Approval_WORKING.md`
  - harness adoption research: `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-governed-action-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-self-settle-lib.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/checks/session-control-runtime-check.mjs`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/docs_repo/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/tests/session-governed-action-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-registry-lib.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-205`
  - `RGF-206`
  - `RGF-209`
- OUTCOME: session-control request/result builders now emit a nested `governed_action` envelope with stable action ids, typed action kinds, and rule-registry-backed policy metadata; broker and self-settle result paths preserve that same action identity through settlement; governed session runtime now keeps a capped `action_history` projection that status and resume surfaces can read directly; and the runtime check validates the new envelope when present while staying backward-compatible with pre-envelope ledger rows

### 2026.04.20.2 / GOV-CHANGE-20260420-02

- STATUS: APPLIED
- SUMMARY: widened `RGF-205` into the validator resume lane by replacing freeform runtime-route resume heuristics with typed `VALIDATOR_GATE_*_RESUME` governed-action envelopes
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-205`
  - typed resume/approval research: `_work/Typed_Runtime_Resume_Approval_WORKING.md`
  - harness adoption research: `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-205`
  - `RGF-206`
  - `RGF-209`
- OUTCOME: validator-governance resume helpers now derive typed governed-action envelopes for runtime-routed approve/defer/skip decisions, `validator-next` consumes those envelopes instead of branching only on boolean readiness/prose, and validator findings now surface the rule-backed resume action that drove the decision; the validator gate ledger format itself remains unchanged for now

### 2026.04.20.3 / GOV-CHANGE-20260420-03

- STATUS: APPLIED
- SUMMARY: widened `RGF-205` into the validator gate ledger so governed gate progression is persisted and read as typed action history instead of only gate-status strings
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-205`
  - typed resume/approval research: `_work/Typed_Runtime_Resume_Approval_WORKING.md`
  - harness adoption research: `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-governed-action-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/validator-gate-governed-action-lib.mjs`
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
  - `.GOV/roles_shared/tests/validator-gate-governed-action-lib.test.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-205`
  - `RGF-206`
  - `RGF-209`
- OUTCOME: validator gate writes now stamp typed governed-action entries for append/commit/present/acknowledge/reset, gate sessions preserve a governed action history and last governed action summary, status and audit readers prefer the typed gate action status over the legacy `session.status` mirror, and `validator-next` ranks/branches off that effective gate status instead of depending only on raw gate-state strings

### 2026.04.20.4 / GOV-CHANGE-20260420-04

- STATUS: APPLIED
- SUMMARY: widened `RGF-205` into the final-lane closeout sync so the terminal Integration Validator sync is recorded as a typed governed external-execute action instead of only a bespoke event row
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-205`
  - typed resume/approval research: `_work/Typed_Runtime_Resume_Approval_WORKING.md`
  - harness adoption research: `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-governed-action-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
  - `.GOV/roles_shared/tests/session-governed-action-lib.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-context-brief-lib.test.mjs`
  - `.GOV/roles_shared/tests/generate-post-run-audit-skeleton.test.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-205`
  - `RGF-206`
  - `RGF-209`
- OUTCOME: the rule registry now covers `INTEGRATION_VALIDATOR_CLOSEOUT_SYNC_EXTERNAL_EXECUTE`, closeout sync events persist the typed governed action result that performed the terminal write, final-lane context/status readers plus the post-run audit skeleton surface that governed action summary instead of reducing terminal sync back to plain mode/status prose, and validator-facing closeout provenance now stays within the same rule-backed action model as session-control transport, validator resume routing, and validator gate progression

### 2026.04.20.5 / GOV-CHANGE-20260420-05

- STATUS: APPLIED
- SUMMARY: widened `RGF-205` into orchestrator-side closeout readers so `phase-check CLOSEOUT` and `closeout-repair` consume the shared typed closeout-governance summary instead of independent terminal-sync interpretation
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-205`
  - typed resume/approval research: `_work/Typed_Runtime_Resume_Approval_WORKING.md`
  - harness adoption research: `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles/orchestrator/scripts/closeout-repair.mjs`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-205`
  - `RGF-206`
  - `RGF-209`
- OUTCOME: the closeout library now exposes one typed closeout-governance summary, `buildIntegrationValidatorCloseoutCheckResult` projects that summary alongside the existing topology/bundle proof, `phase-check CLOSEOUT` uses it when shaping next-command guidance for recorded terminal sync states, and `closeout-repair` surfaces the same governed closeout action during failure diagnosis instead of treating terminal sync provenance as helper-local event prose

### 2026.04.20.6 / GOV-CHANGE-20260420-06

- STATUS: APPLIED
- SUMMARY: completed `RGF-205` by routing the remaining session-control and orchestrator status surfaces through one effective governed session-action projection instead of local `last_command_*` interpretation
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-205`
  - typed resume/approval research: `_work/Typed_Runtime_Resume_Approval_WORKING.md`
  - harness adoption research: `_work/Harness_Adoption_Extraction_WORKING.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-governed-action-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/checks/session-control-runtime-check.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/docs_repo/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/tests/session-governed-action-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-registry-lib.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-206`
  - `RGF-207`
  - `RGF-209`
- OUTCOME: the shared session-governed-action library now derives one effective governed session-action view from typed `action_history` / `last_governed_action` with legacy command fields as fallback only, registry summaries project that effective action directly, active-lane briefs and session-registry status expose it as the primary session-control truth, operator monitor panes render command/action/rule/disposition from the same projection, and the session-control runtime invariant check now validates command mirrors against the effective governed action rather than re-deriving drift locally; with that final reader cutover in place, `RGF-205` is complete

### 2026.04.20.7 / GOV-CHANGE-20260420-07

- STATUS: APPLIED
- SUMMARY: started `RGF-206` by replacing busy `SEND_PROMPT` rejection with durable queued ingress and broker auto-drain semantics across the governed session-control lane
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-206`
  - harness adoption research: `_work/Harness_Adoption_Extraction_WORKING.md`
  - ACP/session-control research: `ACP_Broker_and_Session_Control.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-self-settle-lib.mjs`
  - `.GOV/roles_shared/checks/session-control-runtime-check.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles_shared/schemas/SESSION_CONTROL_REQUEST.schema.json`
  - `.GOV/docs_repo/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-registry-lib.test.mjs`
  - `.GOV/roles_shared/tests/session-control-self-settle-lib.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-206`
  - `RGF-207`
  - `RGF-208`
- OUTCOME: governed `SEND_PROMPT` requests now declare `busy_ingress_mode`, busy ACP ingress persists one durable `pending_control_queue` entry against stable session identity instead of forcing immediate rejection, queued requests survive broker restart and auto-drain after the active run settles, self-settle plus runtime invariant checks now recognize queued requests as legitimate pending state, close-session rejects when queued follow-up work still exists, and operator/session status surfaces expose queue count and next queued request so manual steer/retry loops are replaced by deterministic busy-ingress behavior

### 2026.04.20.8 / GOV-CHANGE-20260420-08

- STATUS: APPLIED
- SUMMARY: continued `RGF-206` by making governed steer and operator status surfaces treat queued busy-session follow-up as first-class truth rather than something to re-infer or resend
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-206`
  - harness adoption research: `_work/Harness_Adoption_Extraction_WORKING.md`
  - ACP/session-control research: `ACP_Broker_and_Session_Control.md`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/lib/orchestrator-steer-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-steer-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/operator-monitor-tui.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/docs_repo/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-206`
  - `RGF-207`
  - `RGF-208`
- OUTCOME: `orchestrator-steer-next` now checks the durable session queue before dispatch and exits successfully when queue-backed follow-up already exists for the governed lane, so re-wake loops stop generating duplicate `SEND_PROMPT` attempts; the operator monitor now counts queued governed work in the top summary, keeps terminal packets visible in the ACTIVE filter when queue-backed work is still pending, and shows queue depth plus queue-head detail in next-action, overview, sessions, and control views so busy-session wait state becomes explicit runtime truth instead of artifact archaeology; `RGF-206` remains open only for queue-aware `orchestrator-next` classification

### 2026.04.20.9 / GOV-CHANGE-20260420-09

- STATUS: APPLIED
- SUMMARY: completed `RGF-206` by making `orchestrator-next` classify queue-backed governed follow-up directly, closing the remaining busy-session resend gap
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-206`
  - harness adoption research: `_work/Harness_Adoption_Extraction_WORKING.md`
  - ACP/session-control research: `ACP_Broker_and_Session_Control.md`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/docs_repo/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-207`
  - `RGF-208`
  - `RGF-209`
- OUTCOME: `orchestrator-next` now reads the governed session registry for the projected next actor, detects when queue-backed follow-up is already accepted for that lane, and reports a deterministic wait state with status/health commands instead of suggesting another relay wake; with `orchestrator-steer-next`, the ACP broker, and the operator monitor already queue-aware, the busy-session resend loop is now closed end-to-end and `RGF-206` is complete

### 2026.04.20.10 / GOV-CHANGE-20260420-10

- STATUS: APPLIED
- SUMMARY: started `RGF-207` by persisting a typed relay escalation policy object in runtime truth and projecting required next-strategy state through the orchestrator relay surfaces
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-207`
  - retry/escalation research: `Harness_Lessons_Learned.md`
  - retry/escalation research: `Technical_Implementation_Research.md`
  - retry/escalation research: `Repo_Governance_Failure_Taxonomy.md`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/lib/wp-relay-watchdog-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles/orchestrator/tests/wp-relay-watchdog-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/templates/WP_RUNTIME_STATUS_TEMPLATE.json`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-207`
  - `RGF-208`
  - `RGF-209`
- OUTCOME: the relay watchdog now derives a typed `relay_escalation_policy` object with `failure_class`, `policy_state`, `next_strategy`, strategy-budget scope, and summary; runtime truth persists that object directly next to the relay/interrupt counters; repair notifications can include the required strategy shift; `session-registry-status`, `wp-relay-watchdog`, and `orchestrator-next` surface the same runtime-native policy so operators stop reconstructing retry state from counters and prose; and the first `RGF-207` slice is in place with the four mechanical strategy classes (`QUEUED_DEFER`, `ALTERNATE_METHOD`, `ALTERNATE_MODEL`, `HUMAN_STOP`) now projected from canonical runtime state

### 2026.04.20.11 / GOV-CHANGE-20260420-11

- STATUS: APPLIED
- SUMMARY: extended `RGF-207` typed relay escalation policy projection into the remaining live operator and lane-diagnostic surfaces
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-207`
  - retry/escalation research: `Harness_Lessons_Learned.md`
  - retry/escalation research: `Repo_Governance_Failure_Taxonomy.md`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs`
  - `.GOV/roles/orchestrator/tests/operator-monitor-tui.test.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles_shared/scripts/session/wp-lane-health.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-207`
  - `RGF-208`
  - `RGF-209`
- OUTCOME: the operator monitor now carries the canonical runtime `relay_escalation_policy` alongside relay status and uses it when summarizing the next governed action, `orchestrator-steer-next` prints the same failure-class / strategy-budget policy before dispatching a governed wake, active-lane briefs project the typed policy in both JSON and formatted text, and `wp-lane-health` reads the same runtime-native policy so blocked auto-recovery shows up as a deterministic diagnostic instead of session-guesswork. The remaining lane-health worktree warning now emits a repo/workspace-relative path (`../wt-...`) rather than a host-specific absolute path, aligning the live diagnostic surface with the orchestrator protocol rule against absolute-path operator output.

### 2026.04.20.12 / GOV-CHANGE-20260420-12

- STATUS: APPLIED
- SUMMARY: completed `RGF-207` by consolidating relay-policy formatting into one shared helper and closing the remaining documentation/status drift
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-207`
  - retry/escalation research: `Harness_Lessons_Learned.md`
  - retry/escalation research: `Technical_Implementation_Research.md`
  - retry/escalation research: `Repo_Governance_Failure_Taxonomy.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-relay-policy-lib.mjs`
  - `.GOV/roles_shared/tests/wp-relay-policy-lib.test.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-lane-health.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/docs_repo/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-208`
  - `RGF-209`
- OUTCOME: relay escalation policy now has one shared normalization/formatting surface, so the watchdog, orchestrator status helpers, operator monitor, active-lane brief, and lane-health diagnostics all read and present the same canonical `failure_class`, `policy_state`, `next_strategy`, and strategy budget. The relay stop path was already runtime-native; this completion tranche removes the remaining helper-local drift and finishes the item with operator-facing docs/runbooks updated to describe the typed repair policy explicitly. `RGF-207` is complete and the next governance tranche is now `RGF-208`.

### 2026.04.20.13 / GOV-CHANGE-20260420-13

- STATUS: APPLIED
- SUMMARY: completed `RGF-208` and `RGF-209` by separating run-vs-step telemetry with durable push-alert projection and collapsing PASS closeout onto one canonical dependency view
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `RGF-208`
  - `RGF-209`
  - telemetry research: `_work/Harness_Adoption_Extraction_WORKING.md`
  - ACP/session-control research: `ACP_Broker_and_Session_Control.md`
  - closeout research: `Validator_Routing_Gates_and_Closeout_Repair.md`
  - workflow-truth research: `Workflow_State_Packet_Truth_and_Range_Drift.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-telemetry-lib.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-lane-health.mjs`
  - `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-closeout-dependency-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/closeout-repair.mjs`
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-closeout-format.mjs`
  - `.GOV/roles_shared/docs/ARCHITECTURE.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
  - `.GOV/docs_repo/GOVERNED_SESSION_CONTROL_ARCHITECTURE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - none in the current tranche
- OUTCOME: governed operator surfaces now distinguish run-level activity from step-level activity and surface the latest durable push alert without reopening terminal transcripts, while final-lane closeout surfaces all consume one shared dependency view with explicit publication truth and blocking keys. The active governance tranche is complete through `RGF-209`.

### 2026.04.20.14 / GOV-CHANGE-20260420-14

- STATUS: APPLIED
- SUMMARY: normalized Cargo artifact-root posture and made `gov-flush` fail early on artifact-root drift
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - `gov-flush` artifact-cleanup failure on stale Cargo `target-dir` posture
  - artifact hygiene policy: `PROJECT_INVARIANTS.md`
  - retention policy: `ARTIFACT_RETENTION_POLICY.md`
- SURFACES:
  - `.GOV/roles_shared/scripts/topology/gov-flush.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`
  - `.GOV/roles_shared/docs/TOOLING_GUARDRAILS.md`
- FOLLOW_ON_ITEMS:
  - none
- OUTCOME: the product worktrees now resolve Cargo artifacts to the canonical `Handshake_Artifacts/handshake-cargo-target` root, `artifact-hygiene-check` and `artifact-cleanup --dry-run` pass again, and `gov-flush` now blocks before any publish step when discovered worktrees drift to a stale Cargo artifact root or repo-local `target/` residue.

### 2026.04.20.15 / GOV-CHANGE-20260420-15

- STATUS: APPLIED
- SUMMARY: refreshed the operator startup cheat sheet for the new lane/path command contract and repaired stale role-model-profile command signatures in the protocols
- CHANGE_TYPE: DOCUMENTATION_HARDENING
- DRIVER_EVIDENCE:
  - `Operator request 2026-04-20`
  - startup cheat sheet drift after recent lane/topology/governance hardening
  - protocol inconsistency on `record-role-model-profiles` argument shape
- SURFACES:
  - `.GOV/operator/docs_local/Handshake_Role_Startup_Prompts.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/activation_manager/ACTIVATION_MANAGER_PROTOCOL.md`
- FOLLOW_ON_ITEMS:
  - none
- OUTCOME: the operator cheat sheet now uses repo-relative workspace anchors instead of host absolute paths, documents `gov-flush`, `classic-orchestrator-startup`, and `orchestrator-steer-next`, clarifies main-only ownership of the canonical root `AGENTS.md` and root `justfile`, and aligns `record-role-model-profiles` with the live five-argument command contract including `ACTIVATION_MANAGER_MODEL_PROFILE`.

### 2026.04.25.01 / GOV-CHANGE-20260425-01

- STATUS: APPLIED
- SUMMARY: made governed ACP/role launch headless-only so role starts and repair launches cannot steal operator keyboard focus
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - Operator request on 2026-04-25: ACP and role launches were hijacking keyboard input by focusing visible windows
  - Calendar Sync v3 workflow-proof startup exposed stale broker/runtime handling at the same control-plane boundary
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles_shared/scripts/session/terminal-ownership-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-governance-state-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-self-settle-lib.mjs`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles/memory_manager/scripts/launch-memory-manager-session.mjs`
  - `.GOV/roles_shared/tests/terminal-ownership-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/session-launch-governance.test.mjs`
  - `.GOV/roles_shared/tests/session-control-self-settle-lib.test.mjs`
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles_shared/docs/ROLE_WORKTREES.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/operator/docs_local/Handshake_Role_Startup_Prompts.md`
  - `.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md`
  - `justfile`
- FOLLOW_ON_ITEMS:
  - none
- OUTCOME: `AUTO` stays on the headless ACP path, `SYSTEM_TERMINAL` repair launches are hidden owned processes, `VSCODE_PLUGIN` governed launches fail closed instead of queueing the VS Code bridge, Memory Manager defaults to the same headless launch path, historical terminal task-board tokens now block governed role starts before runtime artifacts are created, dead-child active runs self-settle, and ACP broker client socket resets are logged instead of crashing the broker.

### 2026.04.26.08 / GOV-CHANGE-20260426-08

- STATUS: APPLIED
- SUMMARY: imported the harness-pattern governance tranche and implemented cache-stability discipline for active role sessions
- CHANGE_TYPE: GOVERNANCE_IMPLEMENTATION
- DRIVER_EVIDENCE:
  - Operator approval on 2026-04-26 to import and implement the harness implementation brief items
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_BRIEFS_20260426_HARNESS_RGFS.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_BRIEFS_20260426_HARNESS_ADDENDUM.md`
  - MT-008 heuristic-loop review note identifying fuzzy repair-loop escalation needs
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/scripts/session/ephemeral-injection-lib.mjs`
  - `.GOV/roles_shared/scripts/session/send-mt-prompt.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs`
  - `.GOV/roles/orchestrator/scripts/lib/manual-relay-envelope-lib.mjs`
  - `.GOV/roles_shared/checks/cache-stability-check.mjs`
  - `.GOV/roles_shared/checks/gov-check.mjs`
  - `.GOV/roles_shared/tests/ephemeral-injection-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/manual-relay-envelope-lib.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-243` through `RGF-250` remain queued from the harness-pattern tranche
  - `RGF-233` through `RGF-241` remain queued from the closeout-canonicalization tranche
- OUTCOME: `RGF-242` is implemented: active-session route, relay, and microtask context now uses a shared `<governance-context>` user-message fence instead of any system-prompt rebuild, the Codex and Orchestrator protocol carry cache-stability law, and `gov-check` now includes `cache-stability-check`.
