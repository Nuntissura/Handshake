# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Product-MT-Task-Board-v1

## STUB_METADATA
- WP_ID: WP-1-Product-MT-Task-Board-v1
- BASE_WP_ID: WP-1-Product-MT-Task-Board
- CREATED_AT: 2026-04-06T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Session-Communication-Database, WP-1-Locus-Work-Tracking-System-Phase1
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: MT task board as a Locus-integrated product feature with DCC visualization
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec MT lifecycle and task tracking
  - Handshake_Master_Spec Locus work tracking system
  - Handshake_Master_Spec 10.11 Dev Command Center
- DISCOVERY_ORIGIN: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION (RGF-102, self-claim task board with complexity-tier routing)

## INTENT (DRAFT)
- What: MT task board as a Locus-integrated product feature. Sessions self-claim MTs from the board. Task status (pending/claimed/completed/failed) is tracked in the product database. Complexity tier routing enables automatic model selection per MT. DCC visualization shows the board in real-time.
- Why: A centralized, database-backed task board replaces ad-hoc MT tracking with a structured system where sessions self-claim work, status is always queryable, and the orchestrator has real-time visibility. Complexity tier routing ensures expensive models are allocated to hard tasks and cheaper models handle straightforward work, directly supporting cost optimization. DCC visualization gives the operator a live view of all MT progress.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - MT task board data model in the product database (via Database trait).
  - MT status lifecycle: PENDING, CLAIMED, IN_PROGRESS, COMPLETED, FAILED, ESCALATED.
  - Session self-claim mechanism: sessions query for unclaimed MTs matching their role and capability.
  - Complexity tier annotation per MT (low/medium/high/xhigh).
  - Automatic model selection based on MT complexity tier.
  - Locus integration: task board state queryable via Locus.
  - DCC real-time task board visualization (frontend component).
  - Flight Recorder events for MT status transitions.
- OUT_OF_SCOPE:
  - MT decomposition logic (orchestrator responsibility).
  - Inter-session communication (see WP-1-Product-Session-Communication-Database-v1).
  - Cost tracking per MT (separate concern).
  - Manual MT creation by operators (orchestrator-generated only for v1).

## ACCEPTANCE_CRITERIA (DRAFT)
- The MT task board stores all MTs for active WPs in the product database with full status lifecycle.
- Sessions can query the board for unclaimed MTs matching their role and self-claim them atomically.
- MT status transitions (PENDING -> CLAIMED -> IN_PROGRESS -> COMPLETED/FAILED/ESCALATED) are enforced and recorded.
- Each MT carries a complexity tier annotation that drives automatic model selection.
- Task board state is queryable through Locus with filters for WP, status, complexity tier, and assigned session.
- The DCC displays a real-time task board view showing all MTs, their status, assigned session, and complexity tier.
- Flight Recorder emits mt-status-changed events for every status transition.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on WP-1-Product-Session-Communication-Database for the database infrastructure and session communication.
- Depends on WP-1-Locus-Work-Tracking-System-Phase1 for Locus query integration.
- Cross-boundary: requires both backend (data model, claim logic) and frontend (DCC visualization).
- No spec blockers identified.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Self-claim race conditions if multiple sessions attempt to claim the same MT simultaneously; needs atomic claim with database-level locking.
- Risk: Complexity tier assignment accuracy depends on orchestrator decomposition quality; misclassified MTs waste resources.
- Risk: Cross-boundary WP increases coordination complexity during implementation.
- Unknown: Whether complexity tier should be reassignable after initial claim (e.g., coder discovers MT is harder than expected).
- Unknown: Optimal DCC refresh rate for real-time board visualization without excessive backend queries.

## DISCOVERY_ORIGIN
- Source: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION
- RGF Reference: RGF-102
- Pattern: Self-claim task board with complexity-tier routing for automatic model selection in multi-agent systems.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Product-MT-Task-Board-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Product-MT-Task-Board-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
