# Non-Coder Governance Stabilization Audit

AUDIT_ID: AUDIT-20260506-NON-CODER-GOVERNANCE-STABILIZATION
STATUS: CLOSED
DATE: 2026-05-06
OWNER: ORCHESTRATOR
SCOPE: Repo Governance

## Driver

The Operator reported that the main mental model is making `ORCHESTRATOR_MANAGED` workflows more mechanical because current governance/workflow is still brittle. A recurring Orchestrator note about mechanical handoffs, stall triage, documentation drift, and autonomous workflow hardening should apply to all governed roles except Coder. Coder must remain focused on product code, while non-Coder roles should actively strive to stabilize governance paperwork and workflow surfaces within their authority.

## Finding

Codex `CX-218K` already required 3-5 plausible-cause triage before patching, steering, relaying, or treating stalls as blocked. Most active non-Coder role protocols already had role-specific `CX-218K` sections. The gap was boundary clarity:

- `protocol-alignment-check.mjs` still enforced CX-218K on `CODER_PROTOCOL.md`, while Coder startup was already excluded from the role-startup CX-218K surface set.
- `STARTUP_BRIEF_SCHEMA.md` said every active role startup brief required the card, which did not reflect the intended Coder exclusion.
- The orchestrator-managed playbook described making future runs easier to babysit instead of reducing Orchestrator babysitting.
- There was no separate Codex rule requiring non-Coder roles to convert repeated governance-paperwork/workflow friction into durable mechanical surfaces.

## Decision

- Add Codex `CX-218L` as the explicit non-Coder governance stabilization duty centered on making `ORCHESTRATOR_MANAGED` workflows more mechanical.
- Add role-specific `Governance Stabilization Duty [CX-218L]` sections to active non-Coder primary protocols.
- Add a Coder exclusion section that keeps Coder in the product-code lane and routes governance blockers through typed handoff/blocker surfaces.
- Add legacy non-Coder agentic add-on notes that inherit `CX-218K`/`CX-218L` only if those add-ons are explicitly re-enabled.
- Update startup brief schema/shared brief and the orchestrator-managed playbook to reflect the non-Coder boundary and reduce Orchestrator babysitting.
- Require non-Coder governance refactor/stabilization work to declare a stable item in `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` and keep that item status current through completion, hold, or supersession.
- Extend `protocol-alignment-check.mjs` so future drift is caught deterministically.

## Primary Surfaces

- `.GOV/codex/Handshake_Codex_v1.4.md`
- `.GOV/roles/*/*PROTOCOL.md`
- `.GOV/roles/*/agentic/AGENTIC_PROTOCOL.md` for non-Coder legacy notes
- `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
- `.GOV/roles_shared/docs/STARTUP_BRIEF_SCHEMA.md`
- `.GOV/roles_shared/docs/SHARED_STARTUP_BRIEF.md`
- `.GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`

## Verification

- `node --check .GOV/roles_shared/checks/protocol-alignment-check.mjs`
- `node .GOV/roles_shared/checks/protocol-alignment-check.mjs`
- `git diff --check`
- `just gov-check`
