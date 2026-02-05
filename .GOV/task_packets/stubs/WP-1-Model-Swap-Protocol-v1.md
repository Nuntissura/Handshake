# Task Packet Stub: WP-1-Model-Swap-Protocol-v1

**Status:** STUB (Not Activated)

## Identity
- WP_ID: WP-1-Model-Swap-Protocol-v1
- BASE_WP_ID: WP-1-Model-Swap-Protocol
- Created: 2026-01-28
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (Handshake_Master_Spec_v02.120.md)

## Roadmap pointer (non-authoritative)
- Handshake_Master_Spec_v02.120.md 7.6.3 (Phase 1) -> MUST deliver (1) Model runtime integration -> [ADD v02.120] sequential model swaps (ModelSwapRequest)

## SPEC_ANCHOR_CANDIDATES (Main Body, authoritative)
- Handshake_Master_Spec_v02.120.md 4.3.3.4.3 ModelSwapRequest (Normative)
- Handshake_Master_Spec_v02.120.md 11.5.6 FR-EVT-MODEL-001..005 (Model Resource Management Events) (Normative) [ADD v02.120]

## Intent (draft)
- What: Implement the ModelSwapRequest protocol and sequential model swaps, including state persistence and ACE recompile before resuming execution.
- Why: Phase 1 runtime integration requires deterministic model resource management and auditable swap events.

## Scope sketch (draft)
- In scope:
  - ModelSwapRequest submission + handling, with persisted swap state and resume semantics.
  - Emit FR-EVT-MODEL-* events as specified; events must be schema-validated at ingestion.
  - Integrate swaps into escalation/role-routing paths that require a different model than currently loaded.
- Out of scope:
  - Adding new model providers beyond existing runtime(s) unless required for swap protocol support.

## Activation checklist (before any coding)
1. In-chat Technical Refinement Block (per .GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md).
2. USER_SIGNATURE.
3. Create .GOV/refinements/WP-1-Model-Swap-Protocol-v1.md.
4. Create official task packet via `just create-task-packet WP-1-Model-Swap-Protocol-v1`.
5. Move Task Board entry out of STUB.


