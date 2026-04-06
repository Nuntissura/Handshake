# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Product-Session-Communication-Database-v1

## STUB_METADATA
- WP_ID: WP-1-Product-Session-Communication-Database-v1
- BASE_WP_ID: WP-1-Product-Session-Communication-Database
- CREATED_AT: 2026-04-06T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Storage-Foundation
- BUILD_ORDER_BLOCKS: WP-1-In-Product-Session-Manager
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Database-backed inter-session communication replacing file-based messaging
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec inter-session communication and message relay
  - Handshake_Master_Spec Database trait and storage abstraction
  - Handshake_Master_Spec Locus system integration
- DISCOVERY_ORIGIN: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION (RGF-101, ACID-guaranteed inter-session messaging pattern)

## INTENT (DRAFT)
- What: Database-backed inter-session communication using the Database trait (SQLite now, PostgreSQL later). Replaces file-based messaging with ACID-guaranteed typed messages, broadcast support, and queryable message history. The communication schema follows the portable pattern (no provider-specific SQL). Integrated with Locus for cross-session message queries.
- Why: File-based inter-session messaging is fragile under concurrent access: race conditions on read/write, no transactional guarantees, no queryable history, and no broadcast semantics. Moving to database-backed communication via the existing Database trait provides ACID guarantees, typed message schemas, broadcast support, and full message history queryable through Locus. This is a tech blocker because reliable inter-session communication is foundational for the in-product session manager.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Communication schema design using portable SQL (no provider-specific features).
  - Typed message enums: DirectMessage, Broadcast, Escalation, StatusUpdate.
  - ACID-guaranteed message send/receive via Database trait.
  - Broadcast support: one-to-many message delivery with delivery tracking.
  - Queryable message history via Locus integration.
  - Message lifecycle: sent, delivered, read, expired.
  - Session inbox/outbox abstraction over the database tables.
  - Migration scripts for the communication schema.
- OUT_OF_SCOPE:
  - The Database trait itself (already exists in WP-1-Storage-Foundation).
  - Real-time push notifications to sessions (polling-based initially).
  - End-to-end encryption of inter-session messages.
  - Cross-machine communication (single-host only for v1).

## ACCEPTANCE_CRITERIA (DRAFT)
- Inter-session messages are stored in the database via the Database trait with ACID guarantees.
- Message types (DirectMessage, Broadcast, Escalation, StatusUpdate) are enforced via typed enums.
- Broadcast messages are delivered to all active sessions with per-session delivery tracking.
- Message history is queryable through Locus with filters for sender, receiver, type, and time range.
- The communication schema uses portable SQL compatible with both SQLite and PostgreSQL.
- Message lifecycle (sent, delivered, read, expired) is tracked per message.
- Migration scripts create and upgrade the communication schema cleanly.
- File-based messaging is fully replaced; no fallback to file-based communication.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on WP-1-Storage-Foundation for the Database trait and storage abstraction layer.
- Blocks WP-1-In-Product-Session-Manager which requires reliable inter-session communication.
- Integrates with Locus for message query capabilities.
- No spec blockers identified.

## RISKS / UNKNOWNs (DRAFT)
- Risk: High-frequency message traffic (many sessions, rapid exchanges) may create database contention on SQLite's single-writer model.
- Risk: Portable SQL constraint may limit query performance optimizations available on specific database backends.
- Risk: Message schema design must anticipate future message types without breaking migrations.
- Unknown: Whether polling-based message delivery provides acceptable latency for real-time session coordination, or whether a notification mechanism is needed from the start.
- Unknown: Optimal message retention policy (keep all history vs. TTL-based expiration).

## DISCOVERY_ORIGIN
- Source: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION
- RGF Reference: RGF-101
- Pattern: ACID-guaranteed database-backed inter-agent communication replacing file-based messaging in multi-agent systems.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Product-Session-Communication-Database-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Product-Session-Communication-Database-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
