# Workflow Contract Registry Audit

AUDIT_ID: AUDIT-20260506-WORKFLOW-CONTRACT-REGISTRY
STATUS: CLOSED
DATE: 2026-05-06
OWNER: ORCHESTRATOR
SCOPE: Repo Governance

## Driver

The Operator clarified that the orchestrator-managed playbook is not a human-facing guide. Roles work blindly and should not carry long prose in context. ACP/session-control needs a machine-readable playbook authored by Orchestrator and Classic Orchestrator so workflow and governance become more mechanical, deterministic, autonomous, transparent, and recoverable.

## Decision

- Add `CX-218M` to Codex: workflow playbooks are machine contracts, not role-memory documents.
- Make `ORCHESTRATOR` owner of `.GOV/roles_shared/workflow_contracts/orchestrator_managed.workflow.json`.
- Make `CLASSIC_ORCHESTRATOR` owner of `.GOV/roles_shared/workflow_contracts/manual_relay.workflow.json`.
- Keep ACP/session-control as consumer/transport only: it receives `workflow_contract` request envelopes and injects compact `WORKFLOW_CONTRACT_CAPSULE` prompts, but does not author policy.
- Keep markdown playbooks as projections/reference surfaces only.
- Validate contracts with `workflow-contract-check.mjs` and include that check in the session bundle.

## Failure Scenarios Captured

The orchestrator-managed contract includes more than 20 machine-readable failure classes, including runtime route drift, notification cursor drift, ACP/session drift, documentation/protocol drift, clock staleness, scope/memory/worktree drift, stale Activation readiness, large-bundle MT compression, projected lane idle, nudge backlog, post-commit relay miss, active run with no output, formatter spillover, wrong review helper, final handoff missing, closeout before verdict, final review route regression, closeout report shape drift, terminal WP rewake, main containment drift, memory scope drift, repeated stall only in chat, packet/task-board drift, and missing worktree preparation.

## Primary Surfaces

- `.GOV/roles_shared/workflow_contracts/orchestrator_managed.workflow.json`
- `.GOV/roles_shared/workflow_contracts/manual_relay.workflow.json`
- `.GOV/roles_shared/schemas/WORKFLOW_CONTRACT.schema.json`
- `.GOV/roles_shared/scripts/workflow/workflow-contract-lib.mjs`
- `.GOV/roles_shared/checks/workflow-contract-check.mjs`
- `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
- `.GOV/roles_shared/schemas/SESSION_CONTROL_REQUEST.schema.json`
- `.GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md`
- `.GOV/codex/Handshake_Codex_v1.4.md`
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
- `.GOV/roles/classic_orchestrator/CLASSIC_ORCHESTRATOR_PROTOCOL.md`

## Verification

- `node --check .GOV/roles_shared/scripts/workflow/workflow-contract-lib.mjs`
- `node --check .GOV/roles_shared/checks/workflow-contract-check.mjs`
- `node .GOV/roles_shared/checks/workflow-contract-check.mjs`
- `node --test .GOV/roles_shared/tests/workflow-contract-lib.test.mjs`
- `node .GOV/roles_shared/checks/protocol-alignment-check.mjs`
- `git diff --check`
- `just gov-check`
