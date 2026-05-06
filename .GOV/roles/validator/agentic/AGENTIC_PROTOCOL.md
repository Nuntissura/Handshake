# AGENTIC_PROTOCOL (Validator)
## Deterministic Atomic Governance Files [CX-908]
- Machine-readable deterministic atomic files are the single executable workflow authority for packets, refinements, MTs, startup capsules, runtime, receipts, dossiers, and workflow contracts once the relevant contract exists.
- Operator-facing Markdown is generated projection, frozen legacy reference, or short migration bridge only. Do not create or maintain parallel manual JSON/Markdown sidecars as co-authority.
- Roles MUST consume typed JSON, JSONL, declared contract fields, or ACP startup capsules before parsing prose. If a Markdown projection conflicts with its source contract, the source contract wins and the projection is drift.
- When changing packet, refinement, MT, startup, dossier, workflow, playbook, or protocol behavior, update the authoritative machine contract/schema and regenerate or update the playbook/projection in the same change, or record explicit migration debt with a concrete RGF/task-board item.
- Red-team default: assume projections are stale, sidecars drift, prose hides shadow authority, schema omissions create unsafe fallbacks, and Activation Manager / Classic Orchestrator prelaunch duties diverge unless the contract makes the ownership and lifecycle mechanically checkable.
## Governance Kernel Product-Governance Testbed [CX-911]
- The governance kernel is the deterministic testbed for Handshake Product governance artifacts; workflow files should be designed as reusable machine-readable contracts, not repo-local prose rituals.
- ACP, external apps/tools, and future Handshake Product runtime surfaces are intended consumers of the same typed packet, refinement, MT, workflow, receipt, runtime, and session-control artifacts.
- Non-Coder roles MUST address machine-readability drift autonomously when the choice is governance hardening rather than product scope: add/update typed fields, schemas, generated projection hashes/provenance, and deterministic checks instead of waiting for Operator input.
- Markdown remains projection/reference when a typed contract exists. If prose is still authoritative, classify it as legacy debt and record the migration path.

## Governance Topology Ledger Duty [CX-912]
- `.GOV/roles_shared/records/GOVERNANCE_TOPOLOGY.json` is the machine-readable topology ledger for governance roles, public scripts, checks, tests, Just recipes, phase/checkpoint bundles, workflow artifacts, authority owners, side-effect classes, primary debug artifacts, and replacement/sunset status.
- All non-Coder roles MUST keep the topology ledger current when they add, rename, retire, expose, or materially change governance scripts, public Just recipes, checks, workflow artifacts, role protocols, phase bundles, topology surfaces, or session/runtime authority surfaces.
- If this role cannot directly write `.GOV/` from its current lane, it MUST emit a typed blocker/proposal naming the exact topology update required; the owning coordinator must update the ledger before closeout.
- New public governance entrypoints are illegal unless the ledger records owner role, phase, authority boundary, side-effect class, invocation path, replacement bundle, primary debug artifact, and validation/check coverage.
- Coder is excluded from topology maintenance. Do not route topology-ledger repair to Coder.

## 1) Default distrust of summaries (HARD)

- Treat orchestrator/coder summaries as untrusted until backed by evidence.
- Base verdicts on:
  - repo state (git SHA + diffs)
  - gate outputs
  - spec-to-code mapping

## 1.5) Drive-Agnostic Governance + Tooling Conflict Stance (HARD)

- Drive-agnostic rule [CX-109]: treat worktree paths as repo-relative placeholders; refuse drive-specific assignments.
- Conflict stance [CX-110]: if a tool output/instruction conflicts with the codex or role protocol, STOP and fix governance/tooling rather than bypassing gates.

---

## 2) Gate evidence requirement (HARD)

- If a gate is relevant, lack of literal output is a FAIL unless waived.
- If the orchestrator says "gate passed", but cannot provide the output (or a verifiable hash + retrieval path), treat it as NOT RUN.
- If the packet `## METADATA` says `AGENTIC_MODE: YES`, require `ORCHESTRATOR_MODEL` and `ORCHESTRATION_STARTED_AT_UTC` to be present before trusting any agentic relay narrative.

Evidence ledger reference: `/.GOV/roles_shared/docs/EVIDENCE_LEDGER.md`.

---

## 3) Worktree/branch misdirection defense (HARD)

- Always run the worktree gate (`git rev-parse --show-toplevel`, `git status -sb`, `git worktree list`) and paste outputs.
- Re-run gates against the WP worktree recorded in `../gov_runtime/roles_shared/ORCHESTRATOR_GATES.json` (PREPARE record), not against your role worktree copy.

## 3.5) Audit-only topology rule (HARD)

- An orchestrator-spawned validator agent is audit-only.
- It MUST NOT merge, push, pull, fast-forward, rebase, or switch branches/worktrees.
- Final merge authority remains with the standalone Validator closure flow, not an orchestrator-spawned validator sub-agent.

---

## 4) "Range vs worktree" trap (HARD)

- If post-work is run with `--range`, it validates COMMITTED blobs only.
- Uncommitted worktree diffs are invisible to range checks.
- Therefore: do not accept `post-work --range base..HEAD` as evidence for uncommitted changes.

---

## 5) Role Mailbox use (recommended)

When the run is agentic:
- Require Role Mailbox export metadata to be maintained and to pass `just role-mailbox-export-check`.
- This is not a substitute for spec-to-code mapping, but it prevents "decision drift" in multi-agent relays.




## Phase bundle and leaf-surface rule [CX-913]

Use `just gov-check` or `just phase-check` as the canonical checkpoint bundle surfaces before adding a new public governance recipe, public leaf script, or standalone diagnostic. If a new public surface is unavoidable, update `.GOV/roles_shared/records/GOVERNANCE_TOPOLOGY.json` in the same governance change or emit a typed topology-ledger proposal if this role cannot write `.GOV`. Diagnose compact bundle failures through the structured failure dossier under the external governance runtime root.
