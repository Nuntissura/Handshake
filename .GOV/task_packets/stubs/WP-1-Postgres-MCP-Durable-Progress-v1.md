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
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Abstraction-Layer, WP-1-Migration-Framework, WP-1-Dual-Backend-Tests
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (dual-backend + MCP durability)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Storage boundary + dual-backend CI posture
  - Handshake_Master_Spec_v02.139.md MCP / tool-call durability requirements (progress token mapping)

## INTENT (DRAFT)
- What: Make MCP durable progress mapping work on Postgres (and portable across DBs) so MCP tool calls do not fail under Postgres.
- Why: Current Postgres backend returns NotImplemented for MCP durable fields; the MCP gate requires these updates, creating a hard protocol failure risk in a dual-backend world.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Add a portable side-table (recommended) to store MCP job fields, avoiding backend-specific ALTER TABLE:
    - `ai_job_mcp_fields(job_id PRIMARY KEY REFERENCES ai_jobs(id), mcp_server_id, mcp_call_id, mcp_progress_token)`
  - Implement the `Database` trait methods for BOTH backends:
    - update_ai_job_mcp_fields(job_id, update)
    - get_ai_job_mcp_fields(job_id)
    - find_ai_job_id_by_mcp_progress_token(token)
  - Add conformance tests that run under sqlite + postgres CI to prevent regressions.
- OUT_OF_SCOPE:
  - Broader MCP schema refactors unrelated to durable progress mapping.

## ACCEPTANCE_CRITERIA (DRAFT)
- Under Postgres, MCP tool-call flows that require progress token durability succeed end-to-end.
- The new schema is migration-backed and portable (no runtime DDL).
- A targeted test fails if Postgres durable mapping is removed or returns NotImplemented.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Migration framework + dual-backend CI already exist.
- Decide whether to keep legacy SQLite-only columns as a compatibility shim or migrate fully to side-table.

## RISKS / UNKNOWNs (DRAFT)
- Backward compatibility: existing SQLite columns may require a one-time data backfill into side-table.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Postgres-MCP-Durable-Progress-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Postgres-MCP-Durable-Progress-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
