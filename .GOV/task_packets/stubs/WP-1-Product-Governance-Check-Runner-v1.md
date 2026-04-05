# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Product-Governance-Check-Runner-v1

## STUB_METADATA
- WP_ID: WP-1-Product-Governance-Check-Runner-v1
- BASE_WP_ID: WP-1-Product-Governance-Check-Runner
- CREATED_AT: 2026-04-05T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Artifact-Registry, WP-1-Unified-Tool-Surface-Contract, WP-1-Capability-SSoT, WP-1-Session-Scoped-Capabilities-Consent-Gate, WP-1-Workflow-Engine, WP-1-Flight-Recorder
- BUILD_ORDER_BLOCKS: WP-1-Governance-Workflow-Mirror, WP-1-Governance-Pack, WP-1-Dev-Command-Center-Control-Plane-Backend
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Backend-first self-hosting split for product governance execution contracts
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md 1.3 The Four-Layer Architecture
  - Handshake_Master_Spec_v02.179.md 6.0.2 Unified Tool Surface Contract
  - Handshake_Master_Spec_v02.179.md 7.5.4 Governance Kernel
  - Handshake_Master_Spec_v02.179.md 10.11 Dev Command Center (Sidecar Integration)

## INTENT (DRAFT)
- What: Define and later implement the governed runner that executes selected imported software-delivery checks, rubrics, and scripts through Handshake runtime.
- Why: Importing repo-governance artifacts is not enough; Handshake also needs a bounded execution contract so software-delivery validation happens through capability-gated, recorder-visible, product-owned workflows instead of raw shell bypass or repo-path coupling.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Executable descriptor contract for imported software-delivery checks, rubrics, and scripts.
  - Capability-gated invocation through Tool Gate and Workflow Engine.
  - Evidence/result schema for PASS, FAIL, BLOCKED, ADVISORY_ONLY, and UNSUPPORTED.
  - Recorder-visible execution lifecycle and bounded artifact capture.
  - Runner policy that makes imported software-delivery governance additive:
    - Handshake-native governance stays broader and remains authoritative outside the software-delivery overlay
- OUT_OF_SCOPE:
  - Blind execution of arbitrary repo scripts.
  - Importing non-deterministic shells as unbounded product authority.
  - UI work beyond minimal inspection/debug surfaces.

## ACCEPTANCE_CRITERIA (DRAFT)
- Product runtime can execute selected imported software-delivery checks/rubrics/scripts through governed tool surfaces.
- Every execution is capability-gated, recorder-visible, and returns a typed result contract.
- Unsupported repo-only artifacts fail closed with explicit reason instead of being silently skipped or treated as product-native law.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on the product governance artifact registry.
- Depends on unified tool surface, capability, workflow, and recorder infrastructure already validated in the product runtime.

## RISKS / UNKNOWNs (DRAFT)
- Risk: a raw shell-driven check runner would recreate repo-governance coupling and bypass Handshake-native controls.
- Risk: imported software-delivery rubrics may encode assumptions that belong in a project profile extension rather than in universal product governance.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Product-Governance-Check-Runner-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Product-Governance-Check-Runner-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
