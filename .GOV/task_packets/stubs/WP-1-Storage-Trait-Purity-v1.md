# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Storage-Trait-Purity-v1

## STUB_METADATA
- WP_ID: WP-1-Storage-Trait-Purity-v1
- BASE_WP_ID: WP-1-Storage-Trait-Purity
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: MEDIUM
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Abstraction-Layer
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (Database trait purity)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Database trait MUST NOT leak backend-specific types
  - Handshake_Master_Spec_v02.139.md Portability + auditability rules (no downcasts to escape hatches)

## INTENT (DRAFT)
- What: Remove/contain backend-type downcasts across the Database boundary and replace them with explicit capability/feature queries.
- Why: Downcasts undermine the boundary and create silent Postgres-vs-SQLite behavioral divergence.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Add explicit backend identity/capability methods to the Database trait (e.g., backend_kind(), supports(feature)).
  - Replace API/backend codepaths that downcast with explicit capability checks.
  - Add a validator/static check to flag new uses of downcast escape hatches.
- OUT_OF_SCOPE:
  - Removing backend-specific code entirely (differences can exist, but must be explicit and audited).

## ACCEPTANCE_CRITERIA (DRAFT)
- No production code paths depend on Database downcasts for behavior decisions.
- Backend-specific behavior is explicit and exercised by tests (sqlite + postgres).

## DEPENDENCIES / BLOCKERS (DRAFT)
- Some features are currently SQLite-only (e.g., Locus); those must be explicitly gated without downcast hacks.

## RISKS / UNKNOWNs (DRAFT)
- Refactor touches many call sites; risk of subtle behavior drift if not backed by targeted tests.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Storage-Trait-Purity-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Storage-Trait-Purity-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
