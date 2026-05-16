# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Postgres-MCP-Durable-Progress-v1

## STUB_METADATA
- WP_ID: WP-1-Postgres-MCP-Durable-Progress-v1
- BASE_WP_ID: WP-1-Postgres-MCP-Durable-Progress
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Abstraction-Layer, WP-1-Migration-Framework
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (PostgreSQL-only MCP durability; older dual-backend posture superseded)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Storage boundary + PostgreSQL-only CI posture
  - Handshake_Master_Spec_v02.139.md MCP / tool-call durability requirements (progress token mapping)

## INTENT (DRAFT)
- What: Make MCP durable progress mapping work on PostgreSQL so MCP tool calls do not fail under PostgreSQL authority.
- Why: Current Postgres backend returns NotImplemented for MCP durable fields; the MCP gate requires these updates, creating a hard protocol failure risk in a PostgreSQL-only world.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Add a PostgreSQL migration-backed side-table (recommended) to store MCP job fields, avoiding runtime ALTER TABLE:
    - `ai_job_mcp_fields(job_id PRIMARY KEY REFERENCES ai_jobs(id), mcp_server_id, mcp_call_id, mcp_progress_token)`
  - Implement the `Database` trait methods for PostgreSQL:
    - update_ai_job_mcp_fields(job_id, update)
    - get_ai_job_mcp_fields(job_id)
    - find_ai_job_id_by_mcp_progress_token(token)
  - Add PostgreSQL conformance tests to prevent regressions. No SQLite CI lane, fixture, fallback, compatibility path, or temporary adapter is allowed.
- OUT_OF_SCOPE:
  - Broader MCP schema refactors unrelated to durable progress mapping.

## ACCEPTANCE_CRITERIA (DRAFT)
- Under Postgres, MCP tool-call flows that require progress token durability succeed end-to-end.
- The new schema is PostgreSQL migration-backed with no runtime DDL.
- A targeted test fails if Postgres durable mapping is removed or returns NotImplemented.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Migration framework and PostgreSQL CI profile exist.
- Do not keep legacy SQLite-only columns as a compatibility shim; migrate to the PostgreSQL side-table path.

## RISKS / UNKNOWNs (DRAFT)
- Backward compatibility: existing legacy SQLite columns, if encountered, are migration/removal evidence only and must not become a runtime or test dependency.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Postgres-MCP-Durable-Progress-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Postgres-MCP-Durable-Progress-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
