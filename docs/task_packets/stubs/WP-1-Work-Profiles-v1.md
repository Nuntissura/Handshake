# Task Packet Stub: WP-1-Work-Profiles-v1

**Status:** STUB (Not Activated)

## Identity
- WP_ID: WP-1-Work-Profiles-v1
- BASE_WP_ID: WP-1-Work-Profiles
- Created: 2026-01-28
- SPEC_TARGET: docs/SPEC_CURRENT.md (Handshake_Master_Spec_v02.120.md)

## Roadmap pointer (non-authoritative)
- Handshake_Master_Spec_v02.120.md 7.6.3 (Phase 1) -> MUST deliver (1) Model runtime integration -> [ADD v02.120] Work Profiles (role-based model assignment + automation knobs)

## SPEC_ANCHOR_CANDIDATES (Main Body, authoritative)
- Handshake_Master_Spec_v02.120.md 4.3.7 Work Profile System (Role-Based Model Assignment + Governance Knobs) (Normative) [ADD v02.120]
- Handshake_Master_Spec_v02.120.md 2.6.6.2.5 Runtime and Models (work_profile_id) (Normative)
- Handshake_Master_Spec_v02.120.md 11.5.9 FR-EVT-PROFILE-001..003 (Work Profile Events) (Normative) [ADD v02.120]

## Intent (draft)
- What: Implement Work Profiles end-to-end (profile storage, selection, per-role model resolution, autonomy knobs) and record work_profile_id in job metadata.
- Why: Phase 1 requires auditable role-based model routing and user-controllable autonomy settings.

## Scope sketch (draft)
- In scope:
  - Work Profile persistence and pin-by-id immutability semantics once used by a job.
  - API + UI hooks for selecting and editing profiles (exact surface finalized in refinement).
  - Emit FR-EVT-PROFILE-* events and ensure event schema validation at Flight Recorder ingestion.
- Out of scope:
  - Adding speculative routing beyond what is required by the normative Work Profile contract.

## Activation checklist (before any coding)
1. In-chat Technical Refinement Block (per docs/ORCHESTRATOR_PROTOCOL.md).
2. USER_SIGNATURE.
3. Create docs/refinements/WP-1-Work-Profiles-v1.md.
4. Create official task packet via `just create-task-packet WP-1-Work-Profiles-v1`.
5. Move Task Board entry out of STUB.

