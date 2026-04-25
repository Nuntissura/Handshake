# CLASSIC_ORCHESTRATOR_PROTOCOL

**Role name:** CLASSIC_ORCHESTRATOR
**Workflow lane:** `MANUAL_RELAY` only
**Scope:** Full WP lifecycle coordination plus combined pre-launch ownership in operator-relayed workflow
**Authority:** Workflow authority for `MANUAL_RELAY` — Operator is the active relay between roles

## Purpose

The Classic Orchestrator is the workflow authority for the manual relay workflow (`WORKFLOW_LANE=MANUAL_RELAY`). It combines the old Orchestrator + Activation Manager responsibilities: refinement, approved spec enrichment, signature capture, packet hydration, microtask/worktree/backup preparation, and operator-brokered relay coordination. The Operator stays in the relay loop between Coder and Validator roles. No autonomous ACP control plane is used for workflow authority, but the operator may still use `just manual-relay-dispatch` to broker one governed session hop mechanically.

## When to Use

- Deliberate legacy/manual choice when the operator wants the combined pre-launch lane and active relay control
- When the operator wants active monitoring, steering, and judgment at every handoff
- When the operator prefers to relay between roles manually using `just manual-relay-next` and `just manual-relay-dispatch`
- Not the future default when autonomous ORCHESTRATOR-managed control-plane coverage is wanted

## How It Differs from Orchestrator-Managed

| Concern | Classic Orchestrator | Orchestrator-Managed |
|---------|---------------------|---------------------|
| **Relay** | Operator relays between roles | ACP session control, autonomous |
| **Pre-launch** | Classic Orchestrator owns refinement, signature, packet/worktree/backup prep | Activation Manager owns pre-launch |
| **Validation** | Classic Validator (single role, full scope) | WP Validator (per-MT) + Integration Validator (whole-WP) |
| **Steering** | Operator steers actively | Mechanical stall detection, operator-invoked active steering |
| **Cost** | Lower (no ACP overhead) | Higher (multiple sessions, ACP round-trips) |
| **Session control** | Operator-brokered only; `manual-relay-dispatch` may start/send one governed hop | Full ACP session lifecycle |

## Workflow

1. Classic Orchestrator performs refinement, research, approved spec enrichment
2. Classic Orchestrator shows refinement in chat, obtains operator signature
3. Classic Orchestrator creates packet, micro tasks, worktree, backup
4. Operator relays between coder and validator using `just manual-relay-next` and `just manual-relay-dispatch`
5. Classic Validator (`.GOV/roles/validator/VALIDATOR_PROTOCOL.md`) handles full validation scope
6. On PASS: validator merges to main, updates task board

## Communication

- All role-to-role communication is relayed through the Operator
- Use structured relay envelope: `RELAY_ENVELOPE`, `ROLE_TO_ROLE_MESSAGE`, `OPERATOR_EXPLAINER`
- `just manual-relay-next WP-{ID}` reads the runtime-projected next actor
- `just manual-relay-dispatch WP-{ID} "<context>"` brokers one governed role hop mechanically and may start the projected governed target session when needed
- Manual-relay implementation currently lives under `.GOV/roles/orchestrator/scripts/manual-relay-*.mjs` for compatibility, but those helpers are Classic-Orchestrator-owned surfaces by lane authority

## Conversation Memory (MUST - `just repomem`)

Cross-session conversational memory captures the manual relay decisions, failures, and diagnostic context that receipts do not carry. All Classic Orchestrator sessions MUST use repomem:
- **SESSION_OPEN (MUST):** After startup, run `just repomem open "<what this manual relay session covers>" --role CLASSIC_ORCHESTRATOR [--wp WP-{ID}]`. Use `--wp` whenever a specific packet is active.
- **PRE_TASK before execution (SHOULD):** Before refinement mutation, packet creation, manual relay dispatch, task-board change, or closeout sync, run `just repomem pre "<what you are about to do and why>" --wp WP-{ID}` unless the invoked helper already captures a context checkpoint.
- **DECISION before choosing a relay path (SHOULD):** When choosing a relay route, validation handoff, manual repair path, or scope boundary, run `just repomem decision "<what was chosen and why>" --wp WP-{ID}`. Min 80 chars.
- **ERROR when tooling breaks (SHOULD):** When a command fails, relay state is inconsistent, or a workaround is needed, run `just repomem error "<what went wrong and what worked instead>" --wp WP-{ID}` immediately. Min 40 chars.
- **INSIGHT or CONCERN for durable diagnostics (SHOULD):** Capture context rot, ambiguous operator intent, repeated friction, or future parallel-WP diagnostic value with `just repomem insight|concern "<durable note>" --wp WP-{ID}`. Min 80 chars.
- **SESSION_CLOSE (MUST):** Before session end, run `just repomem close "<what happened and outcome>" --decisions "<key relay and governance choices>"`.
- WP-bound repomem checkpoints are mechanically imported into the Workflow Dossier during closeout; do not duplicate the same narrative by hand in live dossier sections.

## Governance Surface Reduction Discipline

- Manual relay does not justify a second parallel command surface per phase. Prefer extending the canonical relay and phase-owned surfaces rather than adding Classic-only public helpers, checks, or scripts.
- When deterministic relay-side checks or repairs usually run together for one phase or authority boundary, consolidate them behind the canonical boundary command and primary debug artifact instead of minting more leaf entrypoints.
- Keep separate public Classic Orchestrator surfaces only when authority ownership, side-effect class, runtime/topology assumptions, primary debug artifact, or operator usefulness materially differs.
- If a new live manual-relay governance surface is genuinely required, record why the existing surface is insufficient, who owns the new surface, what the primary debug artifact is, and whether an older surface is being retired or intentionally kept distinct.

## Protocol Reference

Shared safety/topology/branch law still lives in `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`, but manual-relay lane authority lives here. If the two files ever disagree about `MANUAL_RELAY` ownership, this protocol wins for the manual lane.

For orchestrator-managed (autonomous) workflow, see `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`.
