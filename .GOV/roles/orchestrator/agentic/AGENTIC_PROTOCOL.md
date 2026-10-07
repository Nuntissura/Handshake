# AGENTIC_PROTOCOL (Orchestrator)
## Kernel Startup and Authority [ORC-AG-KERNEL]

- [ORC-AG-KERNEL-001] Start every role session in the governance kernel checkout; read its actual root `AGENTS.md` and `CLAUDE.md`, the Codex, this role protocol, and the assigned contract before acting.
- [ORC-AG-KERNEL-002] Read governance directly from the kernel's `.GOV/`; resolve that root explicitly when operating in another checkout. A product-worktree `.GOV` junction is not required.
- [ORC-AG-KERNEL-003] Move to a product checkout only for contract-assigned product execution or inspection; the assigned active WP checkout is canonical for that WP's product work, while `main` is the integrated product baseline and may lag active WPs.
- [ORC-AG-KERNEL-004] Keep all governance state, packets, verdicts, receipts, role/runtime state, task boards, and authority files in the kernel. External `gov-runtime` locations are for tools and caches only, never governance state.
- [ORC-AG-KERNEL-005] Never copy or sync governance into `main` or a product checkout; `main` receives product integration only. Retired legacy WP topology-enforcement clauses do not require governance mirrors, junctions, or main-rooted role startup.
- [ORC-AG-KERNEL-006] Preserve contract scope, role independence, write ownership, validator proof, and product integration acceptance; startup in the kernel grants no additional product or governance write authority.

## Deterministic Atomic Governance Files [CX-914]
- Machine-readable deterministic atomic files are the single executable workflow authority for packets, refinements, MTs, startup capsules, runtime, receipts, dossiers, and workflow contracts once the relevant contract exists.
- Operator-facing Markdown is generated projection, frozen legacy reference, or short migration bridge only. Do not create or maintain parallel manual JSON/Markdown sidecars as co-authority.
- Roles MUST consume typed JSON, JSONL, declared contract fields, or ACP startup capsules before parsing prose. If a Markdown projection conflicts with its source contract, the source contract wins and the projection is drift.
- When changing packet, refinement, MT, startup, dossier, workflow, playbook, or protocol behavior, update the authoritative machine contract/schema and regenerate or update the playbook/projection in the same change, or record explicit migration debt with a concrete RGF/task-board item.
- Red-team default: assume projections are stale, sidecars drift, prose hides shadow authority, schema omissions create unsafe fallbacks, and Activation Manager / Classic Orchestrator prelaunch duties diverge unless the contract makes the ownership and lifecycle mechanically checkable.

## Governance Kernel Product-Governance Testbed [CX-914]
- The governance kernel is the deterministic testbed for Handshake Product governance artifacts; workflow files should be designed as reusable machine-readable contracts, not repo-local prose rituals.
- ACP, external apps/tools, and future Handshake Product runtime surfaces are intended consumers of the same typed packet, refinement, MT, workflow, receipt, runtime, and session-control artifacts.
- Non-Coder roles MUST address machine-readability drift autonomously when the choice is governance hardening rather than product scope: add/update typed fields, schemas, generated projection hashes/provenance, and deterministic checks instead of waiting for Operator input.
- Markdown remains projection/reference when a typed contract exists. If prose is still authoritative, classify it as legacy debt and record the migration path.

## Governance Topology Ledger Duty [CX-912]
Retired 2026-09-24: topology ledger deleted with the harness.



## Phase bundle and leaf-surface rule [CX-913]

Retired with the governance harness on 2026-09-23.
