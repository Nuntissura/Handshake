# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Work Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-FEMS-Pinned-Core-Memory-v1

## STUB_METADATA
- WP_ID: WP-1-FEMS-Pinned-Core-Memory-v1
- BASE_WP_ID: WP-1-FEMS-Pinned-Core-Memory
- CREATED_AT: 2026-04-08T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Front-End-Memory-System
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - §2.6.6.7.6.2.3 MemoryPack schema — item selection and budgets
  - §2.6.6.7.6.2 Design principles — MemoryPack placement, anti-poisoning
  - §4.3.9.12.7 ModelSession FEMS integration — stable prefix placement rules

## INTENT (DRAFT)
- What: Add a `pinned: true` flag to MemoryItem that guarantees inclusion in every MemoryPack before the scoring formula runs on the remaining budget. Ports the Letta/MemGPT "core memory" pattern — items always present in context like RAM.
- Why: Some memories should ALWAYS reach the model: user preferences, safety constraints, project identity, critical procedural rules. Currently MemoryPack is compiled entirely via retrieval scoring, so even critical items can be displaced by high-recency low-importance items. The spec allows procedural items in stable prefix if trusted and review-approved — pinning formalizes this into a first-class mechanism.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - `pinned: boolean` field on MemoryItem schema.
  - Pinned items are included first in MemoryPack compilation, consuming from the token/item budget before scored items are added.
  - Pinned items skip retrieval ranking entirely — they are always present regardless of query scope.
  - Pin/unpin actions routed through MemoryWriteProposal (governed, never implicit).
  - DCC Memory Panel shows pinned items separately with a dedicated view.
  - Budget guard: if pinned items alone exceed the token budget, emit a warning and truncate by pin order (oldest pin first). This is an error state the operator should resolve.
  - Pinned procedural items may be placed in PromptEnvelope stable prefix per §4.3.9.12.7 rules (trust_level=local_authoritative + review-approved).
- OUT_OF_SCOPE:
  - Auto-pinning heuristics (Phase 2 — hygiene manager could suggest pins).
  - Pin inheritance across workspace scopes.

## PILLAR_FORCE_MULTIPLIERS (DRAFT)
- TOUCHED_OR_UNKNOWN_PILLARS:
  - PILLAR: Front End Memory System | STATUS: TOUCHED | NOTES: primary pillar; adds pinning tier to memory taxonomy | Stub follow-up: THIS_STUB
  - PILLAR: Command Center | STATUS: TOUCHED | NOTES: DCC Memory Panel needs pinned items view + pin/unpin controls | Stub follow-up: NONE
  - PILLAR: Spec to prompt | STATUS: TOUCHED | NOTES: pinned procedural items may enter stable prefix of PromptEnvelope | Stub follow-up: NONE
  - PILLAR: ACE | STATUS: TOUCHED | NOTES: WorkingContext compilation must include pinned items before scored retrieval | Stub follow-up: NONE

## ACCEPTANCE_CRITERIA (DRAFT)
- Pinned items always appear in MemoryPack regardless of retrieval query.
- Pinning/unpinning goes through governed MemoryWriteProposal with FR-EVT-MEM-005.
- Pinned items consume budget first; remaining budget used for scored items.
- Budget overflow from too many pins triggers operator-visible warning.
- DCC shows pinned items in dedicated view.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: WP-1-Front-End-Memory-System-v1 for MemoryItem schema and MemoryPack compilation.

## RISKS / UNKNOWNs (DRAFT)
- Risk: over-pinning starves the scored retrieval budget. Need operator guidance on pin budget (e.g., max 30% of token budget for pins).

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-FEMS-Pinned-Core-Memory-v1.md` (approved/signed).
- [ ] Create the official Work Packet via `just create-task-packet WP-1-FEMS-Pinned-Core-Memory-v1`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
