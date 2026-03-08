# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Studio-Runtime-Visibility-v1

## STUB_METADATA
- WP_ID: WP-1-Studio-Runtime-Visibility-v1
- BASE_WP_ID: WP-1-Studio-Runtime-Visibility
- CREATED_AT: 2026-03-08T03:20:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Photo-Studio, WP-1-Atelier-Lens, WP-1-Dev-Command-Center-MVP, WP-1-Locus-Work-Tracking-System-Phase1
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.143.md 7.6.3 (Phase 1) -> [ADD v02.143] Primitive index coverage contract and unresolved Mail/Studio runtime coverage
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.143.md 6.0.2.10 Runtime Visibility Contract (MUST) [ADD v02.142]
  - Handshake_Master_Spec_v02.143.md 6.0.2.11 Primitive Index Coverage Contract (MUST) [ADD v02.143]
  - Handshake_Master_Spec_v02.143.md 10.10 Photo Studio
  - Handshake_Master_Spec_v02.143.md Atelier/Lens runtime sections

## INTENT (DRAFT)
- What: Make Studio/Design Studio surfaces explicit runtime citizens by defining how Studio jobs, tools, Command Center visibility, Flight Recorder evidence, and Locus correlations work across the current Photo Studio and Atelier/Lens surfaces.
- Why: Studio is currently spec-rich and UI-present but under-modeled as a governed runtime surface. That blocks reliable local/cloud tool use, operator visibility, and later matrix expansion.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Canonical runtime mapping for Studio-adjacent surfaces:
    - Canvas / Excalidraw surface.
    - Lens / Atelier collaboration panel.
    - Studio-specific job kinds, workflow nodes, and tool surfaces.
  - Visibility requirements:
    - DCC / operator surface projection.
    - Flight Recorder event linkage.
    - Locus / task-board / WP linkage for Studio-originated work.
  - Storage posture:
    - SQLite-now / PostgreSQL-ready persistence expectations for Studio runtime state and evidence.
- OUT_OF_SCOPE:
  - Full Photo Studio feature completion.
  - Final UI consolidation or visual polish.
  - New creative-tool capabilities beyond the runtime visibility/traceability contract.

## ACCEPTANCE_CRITERIA (DRAFT)
- Studio-adjacent surfaces have explicit job/workflow/tool-call mappings.
- Studio runtime activity is visible in Command Center / operator surfaces and Flight Recorder.
- Locus can correlate Studio-originated work with work packets/microtasks.
- SQLite-now / PostgreSQL-ready posture is specified for Studio runtime state and evidence.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Photo Studio surface and Atelier/Lens runtime behavior must remain the source material for the mapping.
- Depends on DCC visibility and Locus correlation conventions.
- Depends on Flight Recorder evidence conventions for runtime-visible features.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Studio stays fragmented into unrelated UI panels without a canonical runtime identity.
- Risk: operator surfaces show only partial evidence, leading to hidden Studio side effects.
- Risk: storage posture diverges across SQLite and PostgreSQL if the runtime contract is not made explicit early.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Studio-Runtime-Visibility-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Studio-Runtime-Visibility-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
