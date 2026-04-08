# Governance Script Rationalization Matrix

## METADATA
- AUDIT_ID: AUDIT-20260408-GOVERNANCE-SCRIPT-RATIONALIZATION-MATRIX
- SMOKETEST_REVIEW_ID: N/A
- DATE_UTC: 2026-04-08T10:42:42Z
- AUDITOR: Codex ORCHESTRATOR
- SPEC_CURRENT_POINTER: .GOV/spec/SPEC_CURRENT.md
- SPEC_TARGET_RESOLVED: .GOV/spec/Handshake_Master_Spec_v02.180.md
- CODE_TARGET:
  - worktree: `D:\Projects\LLM projects\Handshake\Handshake Worktrees\wt-gov-kernel`
  - branch: `gov_kernel`
  - commit_sha: `168c883cc0edfa4dc72f7424409473708017ea39`
- SCOPE_SUMMARY: First-pass governance script rationalization audit focused on executable and check surfaces only. This excludes deletion or reduction of stubs, work packets, and task packets.
- FOCUS_AREAS:
  - command-surface drift
  - under-tested high-blast-radius scripts
  - compatibility shims and wrapper consolidation
  - session-control and orchestrator script ownership
- RELATED_WP_IDS:
  - NONE
- OUT_OF_SCOPE:
  - deleting stubs
  - deleting work packets or task packets
  - product-code refactors
  - changing governance behavior in this audit
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- RELATED_CHANGESETS:
  - NONE

## METHOD (EVIDENCE-BASED)

- Counted governance script files under `.GOV` using `rg --files` and normalized line counts with a single Node-based line-splitting method.
- Cross-checked each candidate against:
  - live `justfile` recipes
  - direct script imports / wrapper relationships
  - current README / command-surface docs
  - direct `*.test.mjs` references
- Used file:line evidence wherever a positive reference exists.
- Treated missing live `just` exposure as drift only when the script is still mentioned in active governance docs or README surfaces.
- Retirement policy for this cleanup wave: when an old governance script or low-value governance test is removed from the repo, move it to the external archive root `../../scripts_archive/` for safekeeping and posterity instead of hard-deleting it.

## INVENTORY (WHAT EXISTS)

- Governance script surface: `273` files with executable extensions under `.GOV`
- Breakdown:
  - entry scripts: `99`
  - checks: `73`
  - tests: `63`
  - libraries: `36`
  - uncategorized/other: `2`
- `just` command surface: `172` recipes
- Highest script concentrations:
  - `.GOV/roles_shared/scripts/lib`: `28`
  - `.GOV/roles/orchestrator/scripts`: `20`
  - `.GOV/roles_shared/scripts/session`: `20`
  - `.GOV/roles_shared/scripts/topology`: `18`
  - `.GOV/roles/validator/checks`: `17`

## DECISION LEGEND

- `KEEP_CORE_SPLIT_TEST_FIRST`: keep the script, but reduce size and add behavioral tests before any command-surface expansion.
- `KEEP_ADD_TEST`: keep the script, but add explicit tests before further changes.
- `KEEP_SHIM_SHORT_TERM`: keep temporarily as a compatibility entrypoint while the canonical implementation name is normalized.
- `DEFER_LIVE_SURFACE`: actively used through the live `just` surface, governance gates, protocols, or current operator tooling; do not merge/archive in the current wave.
- `DEFER_RECOVERY_SURFACE`: not currently exposed as a normal `just` command, but still present in documented or historical recovery/admin flows; do not retire without a dedicated command-surface review.
- `MERGE_ALIAS`: remove the extra command layer after callers are redirected to the canonical implementation.
- `MERGE_WRAPPER_LATER`: low-risk wrapper consolidation candidate after higher-blast-radius refactors land.
- `RESTORE_OR_RETIRE`: active docs imply the command exists, but the live command surface does not expose it; either restore the supported path or retire the script/doc references.
- `ARCHIVE_OR_REHOME`: no current live command surface; move the file to the external archive root `../../scripts_archive/` or rehome under an explicit admin/debug surface.

## RATIONALIZATION MATRIX

| Priority | File | Lines | Live Surface | Direct Test Refs | Proposed Action | Evidence / Notes |
|---|---|---:|---|---:|---|---|
| P0 | `.GOV/roles/orchestrator/scripts/create-task-packet.mjs` | 1508 | `just create-task-packet` | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | Live activation path via `justfile:559-563`. Imports refinement validation helpers at `create-task-packet.mjs:11-15`. High blast radius, zero direct tests. |
| P0 | `.GOV/roles_shared/checks/refinement-check.mjs` | 2718 | transitively hot | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | Massive shared validator used by packet creation through `create-task-packet.mjs:11-15`. No direct tests found despite centrality. |
| P0 | `.GOV/roles/orchestrator/scripts/operator-monitor-tui.mjs` | 2241 | canonical implementation behind operator viewport | 1 | `KEEP_CORE_SPLIT_TEST_FIRST` | Current implementation file; shim delegates to it at `.GOV/operator/scripts/operator-viewport-tui.mjs:3-8`, while README still calls it a compatibility implementation at `.GOV/roles/orchestrator/README.md:28-29`. |
| P0 | `.GOV/roles/orchestrator/scripts/session-control-command.mjs` | 549 | `just session-start`, `just session-close`, implicit command hub | 1 | `KEEP_CORE_SPLIT_TEST_FIRST` | Generic command multiplexer at `session-control-command.mjs:102-103`; owns `START_SESSION`, `SEND_PROMPT`, `CANCEL_SESSION`, `CLOSE_SESSION` branches around `:129`, `:186`, `:192`, `:302`, `:429`. `justfile:133-143` routes the main session surface through it. |
| P1 | `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs` | 860 | core orchestrator resume path | 2 | `KEEP_CORE_SPLIT_TEST_FIRST` | Central orchestrator control flow; current tests are helper-focused rather than end-to-end behavioral checks in `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs:12-103`. |
| P1 | `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs` | 580 | `launch-*` session commands | 1 | `KEEP_CORE_SPLIT_TEST_FIRST` | Live role-launch path via `justfile:81-86`. High governance coupling, only one direct test reference. |
| P1 | `.GOV/roles/orchestrator/scripts/manual-relay-dispatch.mjs` | 253 | `just manual-relay-dispatch` | 2 | `KEEP_ADD_TEST` | Live via `justfile:287-290`. Captures pre-task snapshots and couples relay envelope generation to session control at `manual-relay-dispatch.mjs:20`, `:25`, `:102`. Tests exist, but coverage remains shallow relative to route complexity. |
| P1 | `.GOV/roles_shared/checks/gov-check.mjs` | 97 | `just gov-check` | 0 | `KEEP_CORE_SPLIT_TEST_FIRST` | Hard-coded check aggregator imports 33 checks in sequence at `gov-check.mjs:33-66`. Central governance gate with zero direct tests. |
| P1 | `.GOV/roles_shared/checks/session-control-runtime-check.mjs` | 507 | `just session-control-runtime-check` | 1 | `KEEP_ADD_TEST` | Live via `justfile:166-167`. Important runtime gate, but direct behavioral coverage is thin for the size. |
| P1 | `.GOV/roles/orchestrator/scripts/role-session-worktree-add.mjs` | 115 | `*-worktree-add` helpers | 0 | `KEEP_ADD_TEST` | Live via `justfile:124-131`. Contains role-specific branching and validator base-branch derivation at `role-session-worktree-add.mjs:55-103` but no direct tests. |
| P2 | `.GOV/operator/scripts/operator-viewport-tui.mjs` | 9 | operator viewport entrypoint | 2 | `DEFER_LIVE_SURFACE` | Live operator entrypoint via `justfile:176-185`, current architecture docs, and runtime-safety tests. It is a shim, but it is still an active surface and should not be merged away mid-run. |
| P2 | `.GOV/roles/orchestrator/scripts/session-control-cancel.mjs` | 27 | `just session-cancel` | 0 | `DEFER_LIVE_SURFACE` | Live command via `justfile:140`; protocol-alignment-check hardcodes this path. Thin wrapper, but currently part of the enforced command surface. |
| P2 | `.GOV/roles_shared/checks/build-order-check.mjs` | 23 | transitively loaded by `gov-check` | 0 | `DEFER_LIVE_SURFACE` | Loaded by `gov-check.mjs:47` and named in `BUILD_ORDER.md:29`. Thin wrapper, but it is in the active governance gate path right now. |
| P2 | `.GOV/roles/orchestrator/scripts/session-control-broker.mjs` | 45 | `just handshake-acp-broker-status/stop` | 0 | `KEEP_ADD_TEST` | Live admin surface via `justfile:169-173`. Small and understandable; add one explicit behavior test before any expansion. |
| P3 | `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs` | 127 | no live `just` recipe found | 0 | `DEFER_RECOVERY_SURFACE` | Still listed in `.GOV/roles/orchestrator/README.md:20` and `.GOV/roles_shared/records/BUILD_ORDER.md:35`. This is drift, but not a safe retirement candidate during active governance usage. |
| P3 | `.GOV/roles/orchestrator/scripts/gov-check-feedback.mjs` | 101 | no live `just` recipe found | 0 | `DEFER_RECOVERY_SURFACE` | No current `just` entrypoint, but it is a purpose-built gov-check failure router with historical workflow usage. Hold until a dedicated recovery/admin-surface review decides whether it is truly dead. |
| P3 | `.GOV/roles/orchestrator/scripts/session-reset-batch-launch-mode.mjs` | 40 | docs-only direct node usage | 0 | `DEFER_RECOVERY_SURFACE` | Referenced in `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md:39` as a batch reset action. Treat as recovery/admin-adjacent and do not retire in the current wave. |
| P3 | `.GOV/roles/orchestrator/scripts/manual-relay-next.mjs` | 123 | `just manual-relay-next` | 2 | `KEEP_ADD_TEST` | Live via `justfile:283-285`. Smaller than dispatch and already referenced by tests, but still part of the manual-lane routing surface and worth keeping under explicit coverage. |

## SUMMARY JUDGMENT

- The main script problem is not raw file count. It is the combination of:
  - a few very large, high-blast-radius scripts with weak direct test coverage
  - command-surface drift where scripts still exist in README / records but not in the live `just` surface
  - compatibility naming layers that obscure where the real implementation lives
- First-pass reduction should therefore proceed in this order:
  1. split and test the P0 monoliths
  2. treat Family A/B surfaces as deferred while they are live or recovery-adjacent
  3. identify genuinely cold scripts outside Family A/B before attempting retirements
  4. normalize naming/ownership only after the live command surface is stable

## RECOMMENDED NEXT ACTIONS

1. Freeze Family A/B from merge/archive work for now; they are live or recovery-adjacent surfaces.
2. Open a narrower usage audit for truly cold scripts that are not in active `just`, gate, protocol, or operator-tooling paths.
3. Refactor `session-control-command.mjs` into subcommands/modules before adding any new session-control behavior.
4. Make `gov-check` registry-driven only after the P0 coverage gaps are closed.
5. Revisit wrapper merges later, once the active command surface is quieter and separately documented.

## COMMAND LOG

- `rg --files .GOV`
- `just --summary`
- targeted `Get-Content` reads for:
  - `justfile`
  - `.GOV/roles/orchestrator/scripts/*.mjs`
  - `.GOV/roles_shared/checks/*.mjs`
  - `.GOV/roles/orchestrator/README.md`
  - `.GOV/roles_shared/records/BUILD_ORDER.md`
  - `.GOV/spec/SPEC_CURRENT.md`
- `git branch --show-current`
- `git rev-parse HEAD`

## DECISIONS / NOTES

- This audit intentionally does not recommend deleting any stubs, task packets, or work packets.
- For script rationalization, retired scripts/tests should be moved to `../../scripts_archive/` rather than hard-deleted.
- Operator review update (2026-04-08): Family A/B scripts are treated as live or recovery-adjacent and are not current merge/archive targets.
- The matrix is a first-pass prioritization artifact, not a claim that every low-surface script should be removed.
- The strongest immediate cleanup wins are structural and test-oriented, not cosmetic file-count reduction.
