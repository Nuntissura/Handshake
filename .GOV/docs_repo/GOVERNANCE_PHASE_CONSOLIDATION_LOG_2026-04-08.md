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

### RGF-158

CLOSEOUT phase-owned sync integration.

Completed:

- `phase-check CLOSEOUT` can now optionally perform the governed closeout truth sync through `--sync-mode ... --context ...`
- when sync is requested, the final memory-manager refresh now runs after that sync inside the same phase artifact
- active helper/docs surfaces now prefer the phase-owned closeout command instead of treating closeout proof and closeout sync as two different public steps

### RGF-159

CLOSEOUT standalone recipe retirement.

Completed:

- the standalone `just integration-validator-closeout-sync ...` recipe was archived and removed from the live `justfile`
- `phase-check CLOSEOUT` now captures its own required repomem context and calls the governed closeout writer directly
- active docs no longer present the standalone closeout sync command as part of the operator-facing surface

Archived in this wave:

- retired recipe definition for:
  - `integration-validator-closeout-sync`

## Topology Correction

The following was clarified after the CLOSEOUT wave:

- `.GOV` is a normal tracked repo directory in `wt-gov-kernel`; it is not a symlink/junction in this worktree
- the repo tracks many files under `.GOV/roles_shared/checks/` and `.GOV/roles_shared/tests/`
- this specific worktree also has a local `.git/info/exclude` rule:
  - `.GOV/*`
  - `!.GOV/docs_repo/`
  - `!.GOV/docs_repo/**`
- because of that local exclude, new or unindexed `.GOV/*` files are hidden from normal `git status` unless they are under `.GOV/docs_repo/**`

Practical consequence:

- the CLOSEOUT replacement implementation currently depends on local edits in:
  - `.GOV/roles_shared/checks/phase-check.mjs`
  - `.GOV/roles_shared/checks/phase-check-lib.mjs`
  - `.GOV/roles_shared/tests/phase-check.test.mjs`
  - `.GOV/roles_shared/tests/governance-command-contract.test.mjs`
- those files are part of tracked repo areas in general, but the specific file entries are not currently in the index on this worktree/branch, so the local exclude hides them
- therefore the CLOSEOUT wave must be treated as a local topology-sensitive cutover until the replacement implementation is brought into tracked topology and committed as real repo truth

This was the main lesson from the session:

- inspect worktree topology, Git tracking, and local excludes first
- do not retire a public phase surface until the replacement is confirmed as tracked in the active topology

## Archive Location

Retired scripts and recipe definitions are being archived under:

- `D:\Projects\LLM projects\Handshake\scripts_archive\`

The current repo-side inventory and replacement mapping is tracked in:

- `.GOV/roles_shared/records/SCRIPT_RATIONALIZATION_LOG.md`

The governance board and change history are tracked in:

- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## What Is True Now

Committed and topology-safe now:

- startup and handoff now have canonical phase-owned public surfaces
- retired startup/handoff shims are archived, not left active
- active docs/helpers have been rebased onto the canonical phase surface
- command drift is mechanically guarded by command-contract and protocol-alignment checks

Local WIP, not yet established as topology-safe committed truth:

- the intended CLOSEOUT end state is still `phase-check CLOSEOUT ... --sync-mode ... --context ...`
- the public recipe retirement and doc updates for CLOSEOUT were done locally
- but the replacement implementation currently lives partly in locally excluded/unindexed `.GOV/roles_shared/checks/*` and `.GOV/roles_shared/tests/*` files, so that cutover is not yet in a clean committed state
- `.GOV/docs_repo/` is now tracked in Git and can be used for this running consolidation log without being hidden by the local exclude

## What Is Not Done Yet

The next meaningful consolidation target is:

- topology-safe CLOSEOUT finalization

Reason:

- the intended CLOSEOUT replacer exists locally, but its implementation is not yet cleanly tracked in the active repo topology
- before any further physical reduction, the replacement implementation must be brought into tracked topology or the live-surface retirement must be reconciled
- only after that is clean should the internal governed writer file be rehomed further for physical file reduction

## Working Principle Going Forward

When consolidating another phase:

- inspect topology first (`git ls-files`, `git check-ignore`, `.git/info/exclude`, worktree layout)
- do not add shims
- do not keep old public aliases alive "just in case"
- build the big replacer first
- ensure the replacer is tracked in the active topology
- archive the retired pieces
- remove the live surface
- make the drift checks enforce the new state

That is the model this consolidation is following.
