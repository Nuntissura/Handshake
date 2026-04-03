# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Storage-Capability-Boundary-Refactor-v1

## STUB_METADATA
- WP_ID: WP-1-Storage-Capability-Boundary-Refactor-v1
- BASE_WP_ID: WP-1-Storage-Capability-Boundary-Refactor
- CREATED_AT: 2026-04-03T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Trait-Purity
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_POINTER: Post-smoketest product follow-on after WP-1-Storage-Trait-Purity-v1; storage boundary factoring to stop trait-capability drift
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md 2.3.13 Storage Backend Portability Architecture [CX-DBP-001]
  - Handshake_Master_Spec_v02.179.md Pillar 1 One Storage API [CX-DBP-010]
  - Handshake_Master_Spec_v02.179.md Pillar 4 Dual-Backend Testing Early [CX-DBP-013]
  - Handshake_Master_Spec_v02.179.md Trait Purity Invariant [CX-DBP-040]

## Why this stub exists
This stub exists because `WP-1-Storage-Trait-Purity-v1` fixed the dangerous part of the old design, but it did so by concentrating more backend capability law and more subsystem-specific methods on the shared `Database` trait.

That was the correct short-term move for removing downcast escape hatches. It is not the right long-term shape for the product. If future feature packets keep solving backend differences by appending more methods and more boolean capability flags to one trait, portability drift will come back in a slower and harder-to-audit form.

## Prior packet
- Prior WP_ID: `WP-1-Storage-Trait-Purity-v1`
- Prior packet: `.GOV/task_packets/WP-1-Storage-Trait-Purity-v1/packet.md`

## Known current gap (Task Board summary)
- / STUB: the product no longer relies on concrete backend downcasts, but the `Database` trait still mixes broad document/canvas/calendar/storage duties with backend capability flags and subsystem-specific hooks for Locus, structured collaboration, Loom, and retention behavior.

## INTENT (DRAFT)
- What: refactor the storage boundary so product subsystems consume narrower capability/domain interfaces rather than accreting more methods onto one monolithic `Database` trait.
- Why: this keeps portability honest at compile time, lowers regression surface, and stops future packets from solving every backend mismatch by widening one trait.

## CURRENT_CODE_SURFACES (DRAFT)
- `src/backend/handshake_core/src/storage/mod.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/storage/retention.rs`
- `src/backend/handshake_core/src/storage/loom.rs`
- `src/backend/handshake_core/src/storage/calendar.rs`
- `src/backend/handshake_core/src/storage/locus_sqlite.rs`
- `src/backend/handshake_core/src/workflows.rs`
- `src/backend/handshake_core/src/api/loom.rs`
- `src/backend/handshake_core/src/storage/tests.rs`

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - define the target boundary shape for storage access, with refinement choosing one honest pattern such as:
    - composed domain stores or subtraits (for example structured collaboration, Loom, calendar, retention, artifact persistence)
    - an explicit backend capability snapshot separated from operational interfaces
    - service-layer handles that expose only the storage surface each subsystem actually needs
  - move subsystem-specific methods off the monolithic `Database` trait where feasible, or isolate them behind dedicated capability objects/trait bundles
  - keep one authoritative storage boundary for callers; do not reintroduce raw pool access, provider-specific types, or concrete backend downcasts
  - migrate representative consumers so they no longer depend on unrelated storage domains:
    - workflow and Locus paths
    - Loom API and observability paths
    - retention or artifact-management paths
    - any other caller the refinement identifies as a high-drift hotspot
  - add tripwires that fail if:
    - concrete backend downcasts return
    - new backend-specific types leak into callers
    - new domain-specific methods are appended to the top-level storage boundary without an explicit allowlisted design decision
- OUT_OF_SCOPE:
  - full feature parity for every backend or every subsystem
  - replacing the storage layer with a completely new architecture
  - user-visible feature work unrelated to the storage boundary

## ACCEPTANCE_CRITERIA (DRAFT)
- The top-level storage boundary is materially smaller or more composition-focused than the current monolithic `Database` trait.
- Representative subsystems no longer depend on unrelated domain methods through one global trait.
- Adding a new backend-specific feature requires touching a dedicated interface or capability contract rather than appending another ad hoc method to `Database`.
- No new concrete-type downcasts or raw provider types leak into callers.
- Dual-backend tests still pass for the migrated storage surfaces.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Storage Trait Purity is already done and provides the safe starting point for this refactor.
- Existing callers are widespread, so refinement must choose a bounded migration sequence instead of trying to redesign the entire storage layer in one pass.
- Some legacy SQLite-only flows may need temporary adapter facades while feature-parity packets catch up.

## RISKS / UNKNOWNs (DRAFT)
- This work can stall if the refinement turns into an unbounded "perfect architecture" rewrite.
- A badly chosen abstraction split can simply recreate the same monolith under a different name.
- Some capability methods may still be the correct boundary in a few places; the packet should remove accidental breadth, not outlaw every explicit capability contract.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Storage-Capability-Boundary-Refactor-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Storage-Capability-Boundary-Refactor-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
