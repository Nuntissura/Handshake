# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-AIReady-CoreMetadata-v1

## STUB_METADATA
- WP_ID: WP-1-AIReady-CoreMetadata-v1
- BASE_WP_ID: WP-1-AIReady-CoreMetadata
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (CoreMetadata required fields)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md CoreMetadata schema requirements (mandatory fields)
  - Handshake_Master_Spec_v02.139.md Bronze/Silver ingestion metadata rules (immutability, provenance)

## INTENT (DRAFT)
- What: Upgrade AI-ready data metadata_json to a versioned CoreMetadata-shaped payload (at least a v0.1 subset) and enforce completeness.
- Why: Current metadata_json stores only basic offsets/path; missing required metadata prevents deep-linking, provenance audits, and consistent retrieval behavior.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define CoreMetadata v0.1 subset (content_id, content_type, hashes, timestamps, owner/source, language, status).
  - Update ingestion pipeline to populate it and validate required fields.
  - Add a migration/backfill strategy if existing records exist.
  - Add tests that fail if required fields are missing.
- OUT_OF_SCOPE:
  - Full multi-source metadata harmonization across every surface (deliver minimal subset first).

## ACCEPTANCE_CRITERIA (DRAFT)
- Newly ingested Bronze/Silver chunks contain CoreMetadata v0.1 fields and pass validation.
- A targeted test fails if metadata_json omits required fields.

## DEPENDENCIES / BLOCKERS (DRAFT)
- A stable content_id policy (deterministic IDs) exists.

## RISKS / UNKNOWNs (DRAFT)
- Backfill and versioning: old records may not have enough data; must define compatibility behavior.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-AIReady-CoreMetadata-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-AIReady-CoreMetadata-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

