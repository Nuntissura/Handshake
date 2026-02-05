# Task Packet Stub: WP-1-AI-Job-Model-v4

**Status:** STUB (Not Activated)

## Identity
- WP_ID: WP-1-AI-Job-Model-v4
- BASE_WP_ID: WP-1-AI-Job-Model
- Created: 2026-01-11
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (currently Handshake_Master_Spec_v02.105.md)

## Why this stub exists
This is an additive remediation stub for `WP-1-AI-Job-Model`.

It is created because the prior packet failed revalidation and/or has audit gaps.

## Prior packet
- Prior WP_ID: `WP-1-AI-Job-Model-v3`
- Prior packet: `.GOV/task_packets/WP-1-AI-Job-Model-v3.md`

## Known gaps (Task Board summary)
- / FAIL (revalidation): `just post-work WP-1-AI-Job-Model-v3` fails phase gate (missing "SKELETON APPROVED" marker); `node .GOV/scripts/validation/post-work-check.mjs WP-1-AI-Job-Model-v3` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.93 not v02.99; packet already contains a prior FAIL section; spec updated in v02.99 to include Stalled and expanded JobKind, revalidate against new list. [READY FOR DEV]

## Activation checklist (before any coding)
1. In-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` Part 2.5.2).
2. USER_SIGNATURE.
3. Create `.GOV/refinements/WP-1-AI-Job-Model-v4.md`.
4. Create official task packet via `just create-task-packet WP-1-AI-Job-Model-v4`.
5. Update `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md` to point `WP-1-AI-Job-Model` -> `WP-1-AI-Job-Model-v4`.
6. Update `.GOV/roles_shared/TASK_BOARD.md` to move `WP-1-AI-Job-Model-v4` out of STUB when activated.

