# AGENTIC_PROTOCOL (Orchestrator)
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

