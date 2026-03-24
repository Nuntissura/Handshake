# DOCUMENTATION_GAPS_AND_NEXT_DOCS

**Status:** Draft  
**Intent:** capture repo-specific documentation gaps and recommended next documentation work  
**Non-goal:** this file is not governance law, protocol law, or an approved implementation roadmap

## Purpose

The governance refactor strengthened the repo materially, but several repo-specific behaviors are now implemented, partially implemented, or actively being refined without one clean documentation surface that explains them end-to-end.

This file records what should likely be documented next and in what order.

It is a documentation backlog, not a ratified policy document.

## How to use this file

- Use this file to decide what documentation to write next.
- Do not treat an item here as adopted repo law unless that rule already exists in a protocol, check, schema, or enforced runtime surface.
- When an item is documented formally, either remove it from this file or mark it as covered by the canonical target document.

## Priority order

## Coverage update

- Priority 1 is now documented in `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`.
- Priority 2 is now documented in `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`.
- Priority 3 is now documented in `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`.
- Priority 4 is now documented in `.GOV/roles_shared/README.md`.
- Priority 5 is now documented in `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`.
- Priority 6 is now documented in `.GOV/roles_shared/docs/ROLE_WORKTREES.md` and `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`.
- Priority 7 is now documented in `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`.
- Priority 8 is now documented in `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`, with navigation links from `START_HERE.md` and `ROLE_WORKFLOW_QUICKREF.md`.

## Priority 1 - Final validator authority

**Why this matters**

The repo distinguishes between `WP_VALIDATOR` and `INTEGRATION_VALIDATOR` in protocol intent, but the final merge-ready authority model still needs clearer executable documentation.

**What should be documented**

- who may issue the final merge-ready verdict
- whether that authority belongs to `WP_VALIDATOR`, `INTEGRATION_VALIDATOR`, `CLASSICAL_VALIDATOR`, or a conditional split
- which gate stores validator role and validator session identity
- which validator actions are intermediate review actions versus final authority actions
- what must exist before merge/push is unlocked

**Best target**

- `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
- supporting executable cross-reference from `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`

## Priority 2 - Exact direct-review contract

**Why this matters**

The repo now enforces stronger coder <-> validator communication rules, including session-targeted receipts and notification handling. Those semantics should be documented directly, not reconstructed from code.

**What should be documented**

- required receipt pairs by workflow stage
- semantics of `correlation_id`
- semantics of `ack_for`
- semantics of `target_session`
- when session identity is required versus optional
- notification acknowledgment behavior
- when unread notifications are blocking
- how `next_expected_actor` and waiting state are derived

**Best target**

- `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
- cross-role examples in `.GOV/roles/coder/CODER_PROTOCOL.md`
- cross-role examples in `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`

## Priority 3 - Session-control repair playbook

**Why this matters**

The orchestrator-managed workflow now has stronger session truth and broker controls, but repair actions still require repo knowledge spread across code and protocol fragments.

**What should be documented**

- stale session demotion
- stale broker state cleanup
- missing-worktree session cleanup
- when to close a governed session
- when to resume a governed session
- when to recreate a session instead of steering an old one
- how to interpret session registry status versus packet status

**Best target**

- new operator runbook section in `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
- optional troubleshooting appendix in `.GOV/roles_shared/docs/RUNBOOK_DEBUG.md`

## Priority 4 - Runtime placement and archival law

**Why this matters**

The repo now enforces a runtime split between repo-local exceptions and external runtime, but that model is still easier to infer from checks than to learn from docs.

**What should be documented**

- what may stay under `/.GOV/roles_shared/runtime/`
- what must live under `../gov_runtime/roles_shared/`
- which artifacts are authoritative
- which artifacts are diagnostic only
- which artifacts are archival only
- how stale runtime residue is removed or archived
- how runtime placement interacts with reviews and audits

**Best target**

- `.GOV/roles_shared/README.md`
- `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
- `.GOV/roles_shared/docs/MIGRATION_GUIDE.md` for legacy cleanup guidance

## Priority 5 - Legacy packet remediation policy

**Why this matters**

The repo now explicitly blocks certain historical packets as remediation-required legacy closures. That migration law should be documented clearly so historical cleanup does not become ad hoc.

**What should be documented**

- when a historical packet is only historical
- when a closed packet becomes `BLOCKED`
- when a new version must be opened instead of mutating the old packet
- what `LEGACY_CLOSED_PACKET_REMEDIATION_REQUIRED` means operationally
- how compatibility shims and sunset rules apply

**Best target**

- `.GOV/roles_shared/docs/MIGRATION_GUIDE.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`

## Priority 6 - Parallel ownership model

**Why this matters**

Handshake is not a single-threaded workflow. The repo should document the intended parallel operating model explicitly instead of leaving it implied by checks and worktree conventions.

**What should be documented**

- multiple coder sessions on disjoint work
- multiple `WP_VALIDATOR` sessions
- handoff to `INTEGRATION_VALIDATOR`
- worktree ownership boundaries
- which concurrent states are allowed
- which concurrent states are blocked
- which states require explicit orchestrator coordination

**Best target**

- `.GOV/roles_shared/docs/ROLE_WORKTREES.md`
- `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`

## Priority 7 - Golden workflow examples

**Why this matters**

The repo now has enough governance machinery that examples would reduce ambiguity for operators, coders, validators, and future check authors.

**What should be documented**

- clean coder -> `WP_VALIDATOR` direct-review cycle
- clean coder -> `WP_VALIDATOR` -> `INTEGRATION_VALIDATOR` cycle
- stale-session recovery
- blocked legacy packet handling
- external-runtime cleanup example

**Best target**

- new examples appendix under `.GOV/roles_shared/docs/`
- short command references in `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`

## Priority 8 - Canonical command surface

**Why this matters**

Several recent sweeps found drift between documented commands and the live `just` surface. That should be prevented by giving the repo one canonical command reference.

**What should be documented**

- live `just` entrypoints by role
- which commands are wrappers versus direct checks
- which commands are governance-kernel only
- which commands require product worktree context
- which commands mutate runtime state

**Best target**

- `.GOV/roles_shared/docs/START_HERE.md`
- `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`

## Suggested documentation rule

When a documentation gap becomes important enough to cause operator confusion, repeated audit findings, or command-surface drift, it should be promoted into a canonical document with:

- one owning document
- one owning protocol or shared doc surface
- explicit cross-links to the enforcing checks or scripts
- examples only where they clarify the law instead of replacing it

## Exit condition for this file

This file is no longer needed once the priority items above are either:

- documented in canonical repo docs
- explicitly rejected as unnecessary
- superseded by a more complete repo governance handbook
