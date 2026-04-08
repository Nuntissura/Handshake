# Governance Phase Consolidation Log

Date: 2026-04-08
Worktree: `wt-gov-kernel`
Branch: `gov_kernel`

## Purpose

This log records what the governance phase consolidation is trying to achieve, why it is being done, what has already been completed, and what remains.

This is not a rationalization-for-its-own-sake effort. The goal is to make governance phase handling mechanical, reduce live surface area, and make each phase easier to run and debug.

## Goal

The intended end state is:

- one real command per phase
- one primary artifact/debug surface per phase
- fewer active `just` commands
- fewer active governance script/check files
- archived retired surfaces outside the live repo
- less documentation drift because fewer public surfaces exist

The target is real surface elimination, not compatibility aliases and not "same public surface, but with libraries underneath".

## Why

The previous model caused repeated remediation because:

- too many narrow commands existed per WP and per phase
- the same workflow was described in many places with copied command strings
- debugging a phase meant hopping across several scripts and outputs
- old commands remained visible after newer composite paths existed

That made both single-WP and parallel-WP operation slower than necessary.

## Consolidation Rule

For each phase:

1. Build the real phase-owned replacement first.
2. Repoint active helpers, prompts, docs, and tests to the canonical phase command.
3. Archive the retired scripts/recipes outside the live repo.
4. Remove the retired live surfaces.
5. Add drift guards so the retired surfaces do not quietly reappear.

## Canonical Public Surface

Current intended public phase surface:

- `just phase-check STARTUP WP-{ID} CODER`
- `just phase-check HANDOFF WP-{ID} CODER`
- `just phase-check HANDOFF WP-{ID} WP_VALIDATOR`
- `just phase-check VERDICT WP-{ID} ...`
- `just phase-check CLOSEOUT WP-{ID}`

The phase runner is:

- `.GOV/roles_shared/checks/phase-check.mjs`

The command builder / phase plan source of truth is:

- `.GOV/roles_shared/checks/phase-check-lib.mjs`

## Completed Waves

### RGF-152

Canonical phase-check command builder and active-doc drift guard.

Completed:

- centralized phase-check command construction
- reduced active command drift around the phase boundary surface

### RGF-153

In-process phase runner and thin wrapper retirement.

Completed:

- `phase-check` became a real in-process runner
- thin wrapper surfaces were retired and archived
- composite phase artifacts became the preferred debugging surface

Archived in this wave:

- `.GOV/roles_shared/checks/active-lane-brief.mjs`
- `.GOV/roles/validator/checks/integration-validator-context-brief.mjs`

### RGF-154

Validator phase leaf rehome and script retirement.

Completed:

- validator handoff and closeout leaf logic moved into existing validator libraries
- `phase-check` now calls those validator surfaces in-process

Archived in this wave:

- `.GOV/roles/validator/checks/validator-handoff-check.mjs`
- `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs`

### RGF-155

Validator packet hygiene rehome and final standalone validator phase-leaf retirement.

Completed:

- packet completeness logic moved into validator governance library
- handoff and closeout consume it in-process

Archived in this wave:

- `.GOV/roles/validator/checks/validator-packet-complete.mjs`

### RGF-156

HANDOFF phase canonical cutover and shim retirement.

Completed:

- coder-side handoff closure now runs through `phase-check HANDOFF ... CODER`
- validator/final helper surfaces route through canonical phase-check commands
- retired HANDOFF shim recipes were removed from the live `justfile`

Archived in this wave:

- `.GOV/roles/coder/checks/post-work.mjs`
- retired recipe definitions for:
  - `post-work`
  - `validator-packet-complete`
  - `validator-handoff-check`
  - `integration-validator-closeout-check`

### RGF-157

STARTUP phase canonical cutover and surface retirement.

Completed:

- coder startup now runs through `phase-check STARTUP ... CODER`
- active protocols, helper scripts, templates, and docs emit the canonical startup command
- startup drift checks now block `just pre-work` and `just gate-check` in active governance surfaces

Archived in this wave:

- `.GOV/roles/coder/checks/pre-work.mjs`
- `.GOV/roles_shared/checks/gate-check.mjs`
- retired recipe definitions for:
  - `pre-work`
  - `gate-check`

## Archive Location

Retired scripts and recipe definitions are being archived under:

- `D:\Projects\LLM projects\Handshake\scripts_archive\`

The current repo-side inventory and replacement mapping is tracked in:

- `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`

The governance board and change history are tracked in:

- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## What Is True Now

After the HANDOFF and STARTUP waves:

- startup and handoff now have canonical phase-owned public surfaces
- retired startup/handoff shims are archived, not left active
- active docs/helpers have been rebased onto the canonical phase surface
- command drift is mechanically guarded by command-contract and protocol-alignment checks

## What Is Not Done Yet

The next meaningful consolidation target is:

- `CLOSEOUT`

Reason:

- CLOSEOUT still has split ownership between phase proof, lifecycle/projection truth-sync, and memory-manager launch
- the next wave should make `phase-check CLOSEOUT` the single authoritative closeout surface

## Working Principle Going Forward

When consolidating another phase:

- do not add shims
- do not keep old public aliases alive "just in case"
- build the big replacer first
- archive the retired pieces
- remove the live surface
- make the drift checks enforce the new state

That is the model this consolidation is following.
