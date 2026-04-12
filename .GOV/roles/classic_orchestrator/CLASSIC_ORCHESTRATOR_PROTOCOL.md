# CLASSIC_ORCHESTRATOR_PROTOCOL

**Role name:** CLASSIC_ORCHESTRATOR
**Workflow lane:** `MANUAL_RELAY` only
**Scope:** Full WP lifecycle coordination in operator-relayed workflow
**Authority:** Workflow authority — Operator is the active relay between roles

## Purpose

The Classic Orchestrator is the coordination role for the manual relay workflow (`WORKFLOW_LANE=MANUAL_RELAY`). In this workflow, the Operator is the active relay between Coder and Validator roles. No ACP session control is used — the Operator manually brokers all role-to-role communication.

## When to Use

- Default for small and medium WPs where autonomous orchestration overhead is not justified
- When the operator wants active monitoring, steering, and judgment at every handoff
- When the operator prefers to relay between roles manually using `just manual-relay-next` and `just manual-relay-dispatch`

## How It Differs from Orchestrator-Managed

| Concern | Classic Orchestrator | Orchestrator-Managed |
|---------|---------------------|---------------------|
| **Relay** | Operator relays between roles | ACP session control, autonomous |
| **Pre-launch** | Orchestrator owns refinement, packet creation | Activation Manager owns pre-launch |
| **Validation** | Classic Validator (single role, full scope) | WP Validator (per-MT) + Integration Validator (whole-WP) |
| **Steering** | Operator steers actively | Mechanical stall detection, operator-invoked active steering |
| **Cost** | Lower (no ACP overhead) | Higher (multiple sessions, ACP round-trips) |
| **Session control** | None — operator manages terminals | Full ACP session lifecycle |

## Workflow

1. Orchestrator performs refinement, research, spec enrichment
2. Orchestrator shows refinement in chat, obtains operator signature
3. Orchestrator creates packet, micro tasks, worktree, backup
4. Operator relays between coder and validator using `just manual-relay-next` and `just manual-relay-dispatch`
5. Classic Validator (`.GOV/roles/validator/VALIDATOR_PROTOCOL.md`) handles full validation scope
6. On PASS: validator merges to main, updates task board

## Communication

- All role-to-role communication is relayed through the Operator
- Use structured relay envelope: `RELAY_ENVELOPE`, `ROLE_TO_ROLE_MESSAGE`, `OPERATOR_EXPLAINER`
- `just manual-relay-next WP-{ID}` reads the runtime-projected next actor
- `just manual-relay-dispatch WP-{ID} "<context>"` brokers one governed role hop mechanically

## Protocol Reference

The full orchestrator protocol (safety rules, branch model, governance folder structure, packet rules) is defined in `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`. The MANUAL_RELAY sections of that protocol apply to this role.

For orchestrator-managed (autonomous) workflow, see the Orchestrator Role Definition block in ORCHESTRATOR_PROTOCOL.md.
