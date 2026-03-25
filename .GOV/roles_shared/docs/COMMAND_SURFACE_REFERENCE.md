# COMMAND_SURFACE_REFERENCE

**Status:** Draft  
**Intent:** canonical operator-facing `just` command surface for current governance workflow  
**Authority note:** the live executable surface is the root `justfile`; if this document disagrees with `just --list`, treat this document as stale and fix it
**Startup prompt note:** this document is an operator cheat sheet only. Governed role startup prompts must be derived from `.GOV/roles_shared/scripts/session/session-control-lib.mjs::buildStartupPrompt`, not hand-maintained here.

## Purpose

This file groups the live `just` surface by workflow purpose so roles do not have to infer command meaning from protocol prose alone.

For the exhaustive inventory, run:

```bash
just --list
```

## Reading key

- `read-only`: does not intentionally mutate packet/runtime state
- `runtime-write`: mutates external runtime ledgers, broker state, session state, or packet communication state
- `governance-write`: mutates governed files under `/.GOV/`
- `product-scan`: reads product code and is expected to run from the appropriate product worktree context

## Operator-facing scope split

Use this split in chat every time scope, remediation, or next steps are discussed:

- `Handshake (Product)`
  - product code, product tests, Master Spec requirements, product WPs
  - includes product-governance contract work such as governed actions, queue law, workflow-state semantics, or runtime contract enforcement when the diff touches `src/`, `app/`, `tests/`, or the Master Spec
- `Repo Governance`
  - `/.GOV/**`, ACP/session/runtime ledgers, role protocols, governance task-board/changelog/audits, root control-file maintenance
  - this lane does not use a WP when the planned diff stays governance-only
- If only one lane applies, still name both lanes and state `NONE` for the other lane.

## Governance health and read-only status

These are safe starting points for orientation and health checks.

- `just gov-check`
  - `read-only`
  - shared governance health; expected before new governed execution and before final closure
- `just docs-check`
  - `read-only`
  - presence check for required governance docs
- `just session-registry-status [WP-{ID}]`
  - `read-only`
  - inspect governed session state
- `just handshake-acp-broker-status`
  - `read-only`
  - inspect ACP broker liveness/state
- `just operator-monitor`
  - `read-only`
  - operator viewport across sessions, receipts, control results, and packet/runtime activity
- `just orchestrator-next [WP-{ID}]`
- `just coder-next [WP-{ID}]`
- `just validator-next [WP-{ID}]`
  - `read-only`
  - role-specific resume helpers after startup/reset/compaction

## Minimal Live Read Set (Token Discipline)

After startup and assignment, roles should usually be able to operate from a small live read set:

- startup output
- the assigned packet
- the active WP thread / notifications
- this command-surface reference when command choice is unclear

Repeated full rereads of large governance protocols, repeated `just --list`-style command rediscovery, and repeated path/source-of-truth checks after context is already stable should be treated as ambiguity smells, not as normal diligence.

If a role keeps needing those rereads:

- prefer tightening the startup prompt, packet, command surface, or helper command
- record the churn in the next smoketest review under `Silent Failures, Command Surface Misuse, and Ambiguity Scan`

## Startup and preflight

- `just orchestrator-startup`
- `just coder-startup`
- `just validator-startup`
  - `read-only`
  - protocol ack + backup context + role preflight
- `just orchestrator-preflight`
- `just coder-preflight`
- `just validator-preflight`
  - `read-only`
  - compact gate bundles for startup
- `just hard-gate-wt-001`
  - `read-only`
  - print worktree/branch verification block for chat

## Governance maintenance (no WP)

Use this flow only for repo-governance maintenance that stays out of product code and the Master Spec.

- Working records:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `.GOV/Audits/**` with stable `AUDIT_ID` and, for smoketest reviews, `SMOKETEST_REVIEW_ID`
- Working templates:
  - `.GOV/templates/REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`
  - `.GOV/templates/REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
- Shared workflow note:
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
- Commands:
  - `just gov-check`
    - `read-only`
    - mandatory verification before claiming governance-maintenance completion
  - `just build-order-sync`
    - `governance-write`
    - required only when governance changes affect `TASK_BOARD.md` or `WP_TRACEABILITY_REGISTRY.md`

## Packet activation and governance state writes

These mutate packet, board, traceability, or related governed surfaces.

- `just record-refinement WP-{ID}`
- Refinement visibility rule:
  - the Operator must see the refinement in assistant-authored chat text before any signature request
  - shell/tool output does not count as "shown in chat"
  - if the refinement is too large for one message, paste it verbatim across multiple consecutive chat messages before requesting approval
- `just record-signature WP-{ID} <signature> <workflow_lane> <execution_lane>`
- `just record-prepare WP-{ID} [workflow_lane] [execution_lane] [branch] [worktree_dir]`
  - `governance-write`
  - orchestrator-owned workflow state writes
- `just create-task-packet WP-{ID}`
  - `governance-write`
  - packet creation from the template
- `just orchestrator-prepare-and-packet WP-{ID}`
  - `governance-write`
  - transactional prepare + packet creation + sync flow
- `just task-board-set WP-{ID} <STATUS> ["reason"]`
  - use `DONE_MERGE_PENDING` after validator PASS append but before merge-to-main containment
  - use `DONE_VALIDATED` only after the approved closure commit is actually contained in local `main`
- `just build-order-sync`
  - `governance-write`
  - projection updates
- `just post-run-audit-skeleton WP-{ID} [output]`
  - `read-only`
  - generate audit skeleton from current authoritative artifacts

## Session launch and steering (Orchestrator-only)

These mutate governed runtime state and should not be run from inside Coder or Validator sessions.
For Orchestrator-managed WPs, this ACP/CLI session surface is the required normal delegation path.
Helper agents/subagents may assist on governance/spec/runtime/orchestrator tasks, but they are not Coder or Validator lanes.
Do not use helper agents/subagents for Coder or Validator duties, and do not let them write product code, unless the Operator explicitly approved that path and the packet records `SUB_AGENT_DELEGATION: ALLOWED` plus exact `OPERATOR_APPROVAL_EVIDENCE`.

- `just launch-coder-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-wp-validator-session WP-{ID} ...`
- `just launch-integration-validator-session WP-{ID} ...`
  - `runtime-write`
  - launch/bootstrap lane
- `just start-coder-session WP-{ID} [PRIMARY|FALLBACK]`
- `just start-wp-validator-session WP-{ID} ...`
- `just start-integration-validator-session WP-{ID} ...`
  - `runtime-write`
  - first governed ACP start for the lane
- `just steer-coder-session WP-{ID} "<prompt>" [PRIMARY|FALLBACK]`
- `just steer-wp-validator-session WP-{ID} ...`
- `just steer-integration-validator-session WP-{ID} ...`
  - `runtime-write`
  - governed ACP resume/send
- `just cancel-coder-session WP-{ID}`
- `just cancel-wp-validator-session WP-{ID}`
- `just cancel-integration-validator-session WP-{ID}`
  - `runtime-write`
  - cancel the current governed command for that lane
- `just close-coder-session WP-{ID}`
- `just close-wp-validator-session WP-{ID}`
- `just close-integration-validator-session WP-{ID}`
  - `runtime-write`
  - retire steerable thread registration for that lane
- Generic wrappers:
- `just session-start <ROLE> WP-{ID} [PRIMARY|FALLBACK]`
- `just session-send <ROLE> WP-{ID} "<prompt>" [PRIMARY|FALLBACK]`
- `just session-cancel <ROLE> WP-{ID}`
- `just session-close <ROLE> WP-{ID}`
    - these governed helpers now attempt deterministic self-settlement for their own request ids when a broker dispatch or wait path returns without a terminal result row

## Packet communication surface

These operate on the packet-declared `WP_COMMUNICATION_DIR` under external runtime.

- `just wp-thread-append ...`
  - `runtime-write`
  - soft coordination only
- `just wp-heartbeat ...`
  - `runtime-write`
  - liveness/state projection only
- `just wp-receipt-append ...`
  - `runtime-write`
  - low-level deterministic receipt append
- `just wp-invalidity-flag WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <INVALIDITY_CODE> ...`
  - `runtime-write`
  - records a machine-visible `WORKFLOW_INVALIDITY` receipt and routes attention back to the Orchestrator
- `just wp-validator-kickoff ...`
- `just wp-coder-intent ...`
- `just wp-coder-handoff ...`
- `just wp-validator-review ...`
- `just wp-validator-response ...`
- `just wp-review-response ...`
- `just wp-review-exchange <RECEIPT_KIND> ...`
- `just wp-spec-gap ...`
- `just wp-spec-confirmation ...`
  - `runtime-write`
  - structured direct-review / review-resolution helpers
- `just wp-communication-health-check WP-{ID} [STATUS|KICKOFF|HANDOFF|VERDICT]`
  - `read-only`
  - communication proof and route health
- `just check-notifications WP-{ID} <ROLE>`
  - `read-only`
  - inspect unread session-targeted notifications
- `just ack-notifications WP-{ID} <ROLE> <session>`
  - `runtime-write`
  - acknowledge notifications for one governed session only

## Coder execution surface

These are typically run from the WP-assigned worktree.

- `just pre-work WP-{ID}`
  - `read-only`
  - blocking packet-integrity/start gate
- `just post-work WP-{ID}`
  - `read-only`
  - deterministic closure gate against the validated diff window
- `just coder-skeleton-checkpoint WP-{ID}`
- `just skeleton-approved WP-{ID}`
  - `governance-write`
  - docs-only phase-boundary helpers for `MANUAL_RELAY` only
  - forbidden on `ORCHESTRATOR_MANAGED`; those invocations now fail and record `WORKFLOW_INVALIDITY`
- `just product-scan`
- `just validator-dal-audit`
- `just validator-git-hygiene`
  - `product-scan`
  - coder hygiene surface before handoff
- `just cargo-clean`
  - `product-scan`
  - workspace cleanup targeting `handshake_core`

## Validator execution surface

These are usually run from the WP worktree for WP-validator work or from `handshake_main` for integration-validator/final validation work.

- `just gate-check WP-{ID}`
- `just validator-handoff-check WP-{ID}`
- `just integration-validator-closeout-check WP-{ID}`
- `just validator-packet-complete WP-{ID}`
- `just wp-declared-topology-check WP-{ID}`
- `just validator-policy-gate WP-{ID}`
    - `read-only`
    - primary validator gate surface
    - `integration-validator-closeout-check` is the final-lane topology and atomic-closeout preflight for orchestrator-managed PASS closure
    - `wp-declared-topology-check` surfaces packet-declared vs actual linked-worktree truth for one WP and fails on undeclared auxiliary worktrees
  - for `PACKET_FORMAT_VERSION >= 2026-03-25`, `Done` means merge-pending PASS and `Validated (PASS)` requires recorded containment in local `main`
- `just external-validator-brief WP-{ID}`
  - `read-only`
  - classical/external validation contract summary
- `just validator-gate-append WP-{ID} <PASS|FAIL|...>`
- `just validator-gate-commit WP-{ID}`
- `just validator-gate-present WP-{ID} [verdict]`
- `just validator-gate-acknowledge WP-{ID}`
- `just validator-gate-reset WP-{ID} <confirm>`
  - `governance-write`
  - validator gate state progression/reset
- `just validator-gate-status WP-{ID}`
- `just validator-governance-snapshot`
- `just validator-report-structure-check`
- `just validator-phase-gate [Phase-n]`
  - `read-only`
  - governance-oriented validator status/report checks
- `just validator-scan`
- `just validator-error-codes`
- `just validator-coverage-gaps`
- `just validator-traceability`
- `just validator-hygiene-full`
  - `product-scan`
  - product-side audit and hygiene family

## Worktree and topology management

These are orchestrator/operator/topology commands, not ordinary coder commands.

- `just worktree-add WP-{ID}`
- `just coder-worktree-add WP-{ID}`
- `just wp-validator-worktree-add WP-{ID}`
- `just integration-validator-worktree-add WP-{ID}`
  - `runtime-write`
  - create/repair worktree mappings or role session worktree context
- `just backup-status`
  - `read-only`
  - backup health visibility
- `just backup-push <local_branch> <remote_branch>`
- `just backup-snapshot [label]`
  - `runtime-write`
  - safety preservation surfaces
- `just sync-all-role-worktrees`
- `just reseed-permanent-worktree-from-main <worktree_id> "<approval>"`
- `just sync-gov-to-main`
  - `governance-write`
  - topology refresh / governance propagation
- `just enumerate-cleanup-targets`
  - `read-only`
  - deterministic cleanup preview
- `just generate-worktree-cleanup-script WP-{ID} <ROLE>`
  - `read-only`
  - produce hard-bound cleanup script
- `just delete-local-worktree <worktree_id> "<approval>"`
- `just close-wp-branch WP-{ID} "<approval>" [remote]`
- `just retire-standalone-checkout <checkout_id> "<approval>"`
  - `runtime-write`
  - approved destructive/retirement surfaces

## Command-surface rules

- Prefer the role-specific wrapper when one exists.
  - Example: use `just start-wp-validator-session ...` instead of the generic `just session-start WP_VALIDATOR ...` unless you specifically need the generic form.
- `product-scan` is an alias for `validator-scan`.
- `THREAD.md` commands are not substitutes for required structured review receipts.
- A command being available in `just --list` does not mean every role may run it. Role protocols still define ownership.
- Any command not present in the current `just --list` output should be treated as retired, stale, or a documentation bug until restored.
