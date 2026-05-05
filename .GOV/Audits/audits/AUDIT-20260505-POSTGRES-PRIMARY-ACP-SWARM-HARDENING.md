# AUDIT-20260505-POSTGRES-PRIMARY-ACP-SWARM-HARDENING

## Metadata

- AUDIT_ID: AUDIT-20260505-POSTGRES-PRIMARY-ACP-SWARM-HARDENING
- STATUS: ACTIVE
- CREATED_AT: 2026-05-05T17:55:00Z
- OWNER: ORCHESTRATOR
- GOVERNANCE_ITEM: RGF-281
- SCOPE: Repo Governance
- PRODUCT_WP_CONTEXT: WP-1-Postgres-Primary-Control-Plane-Foundation-v1

## Driver

The Operator started `WP-1-Postgres-Primary-Control-Plane-Foundation-v1` as an orchestrator-managed ACP workflow, declared GPT-5.5 extra-high for all governed roles except WP Validator on Claude Opus 4.7 extra thinking, warned that high-volume scripts and host load may be active, and asked the Orchestrator to babysit ACP while hardening the workflow for future swarm-parallel work packets.

## Scope

In scope:
- ACP/session-control runtime checks for accepted/running/queued states under heavy load.
- Activation Manager handoff and duplicate-stub/refinement responsibility boundaries.
- Packet/role model-profile self-checking for GPT-5.5 and Claude Opus 4.7 profile IDs.
- Repomem coverage debt visibility during long-running orchestrator-managed sessions.
- Fallback/repair guidance for stale, stalled, queued, or busy ACP sessions.

Out of scope:
- Product code under `src/`, `app/`, or `tests/`.
- Master Spec text or PostgreSQL product implementation.
- Replacing WP Validator or Integration Validator technical authority.
- Destructive cleanup or worktree deletion.

## Initial Runtime Evidence

- Activation Manager `START_SESSION` for `WP-1-Postgres-Primary-Control-Plane-Foundation-v1` was accepted as running under `HANDSHAKE_ACP_BROKER`.
- The follow-up stub instruction was accepted as queued behind the active Activation Manager run.
- The operator override assigning stub creation to Orchestrator was also accepted as queued.
- `session-registry-status` surfaced repomem coverage debt and token-ledger drift as diagnostic/runtime truth while the role session was still starting.
- The historical launch batch mode already reflected plugin instability and CLI escalation context, so heavy-load checks must distinguish current-session health from historical batch mode.

## Candidate Hardening Targets

- Accepted/queued state duplicate suppression: prevent repeated sends when ACP already accepted work.
- Activation Manager handoff split: make refinement/spec-enrichment ownership mechanically distinct from Orchestrator-created stub backlog work.
- Model-profile enforcement: fail fast when packet/session profile IDs drift from operator-declared role profiles.
- Repomem coverage debt: keep debt visible without treating startup-phase debt as final session failure.
- Heavy-load fallback checks: distinguish busy, stalled, timed out, accepted-running, and terminal states in operator-facing commands.
- Parallel WP monitoring: make multi-WP status summaries show next expected actor/session, queued commands, and stale-age in one primary surface.

## Implemented Slice 2026-05-05

- `worktree-concurrency-check.mjs` now treats active `ACTIVATION_MANAGER`, `ORCHESTRATOR`, and `CLASSIC_ORCHESTRATOR` sessions as prelaunch workflow-authority lanes that do not by themselves require coder/WP-validator worktree mappings before packet activation.
- Task Board `IN_PROGRESS` entries still require dedicated WP worktree mappings, so executable product work keeps the one-WP/one-worktree guard.
- Focused tests cover role-level exclusion and the rule that an `IN_PROGRESS` Task Board entry for the same WP re-enables the dedicated-worktree requirement.

## Verification Plan

- Run focused ACP/session tests after implementation patches.
- Run `just session-registry-status WP-1-Postgres-Primary-Control-Plane-Foundation-v1` during the active workflow.
- Run `just gov-check` before any governance commit that changes live governance surfaces.
