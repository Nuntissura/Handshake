# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Product-Governance-Artifact-Registry-v1

## STUB_METADATA
- WP_ID: WP-1-Product-Governance-Artifact-Registry-v1
- BASE_WP_ID: WP-1-Product-Governance-Artifact-Registry
- CREATED_AT: 2026-04-05T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Snapshot, WP-1-Governance-Kernel-Conformance, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: WP-1-Product-Governance-Check-Runner, WP-1-Governance-Workflow-Mirror, WP-1-Governance-Pack, WP-1-Dev-Command-Center-Control-Plane-Backend
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Backend-first self-hosting split for product governance overlay import
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.179.md 1.3 The Four-Layer Architecture
  - Handshake_Master_Spec_v02.179.md 7.5.4 Governance Kernel
  - Handshake_Master_Spec_v02.179.md 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)
  - Handshake_Master_Spec_v02.179.md 10.11 Dev Command Center (Sidecar Integration)
  - Handshake_Master_Spec_v02.179.md [ADD v02.167] / [ADD v02.168] structured collaboration base-envelope and project-profile extension rules

## INTENT (DRAFT)
- What: Define and later implement a product-owned registry for imported repo-governance artifacts: codex, role protocols, rubrics, check manifests, script descriptors, schemas, templates, and sync metadata.
- Why: Handshake needs a bounded, versioned way to ingest the current repo governance surface without turning repo file paths into runtime authority or flattening Handshake's broader governance stack into software-delivery-only rules.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Versioned registry and manifest format for imported software-delivery governance artifacts.
  - Stable artifact kinds and identities for codex, protocol, rubric, script, check, template, schema, and sync-surface descriptors.
  - Product-owned storage and load rules for the imported overlay.
  - Additive overlay rules:
    - imported software-delivery governance augments Handshake-native governance
    - imported overlay MUST NOT replace or hide broader Handshake-native governance layers or project-profile contracts
  - Provenance metadata that records source snapshot/version/hash for imported governance artifacts.
- OUT_OF_SCOPE:
  - Executing imported checks or scripts.
  - Replacing Handshake-native governance with repo-governance files.
  - UI shells for browsing the registry beyond minimal inspection/debug surfaces.

## ACCEPTANCE_CRITERIA (DRAFT)
- Product runtime can materialize a bounded imported-governance registry without reading repo `.GOV/**` as live authority.
- Imported software-delivery governance artifacts are versioned, typed, and provenance-linked.
- The registry can coexist with non-software Handshake governance layers and project-profile extensions without field collisions or naming overrides.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on product governance boundary work already completed in `WP-1-Product-Governance-Snapshot-v4`.
- Depends on the structured collaboration artifact family and schema registry so imported governance records can live inside product-owned canonical artifacts instead of ad hoc file copies.

## RISKS / UNKNOWNs (DRAFT)
- Risk: treating repo-governance imports as the whole product governance layer would overwrite Handshake-native governance instead of extending it.
- Risk: importing too much raw repo layout detail would reintroduce repo-path authority and block future non-software kernels.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Product-Governance-Artifact-Registry-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Product-Governance-Artifact-Registry-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
