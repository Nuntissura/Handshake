# Task Packet Stub: WP-1-Photo-Studio-v2

**Status:** SUPERSEDED / DEPRECATED (Folded into `WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1`)

## Identity
- WP_ID: WP-1-Photo-Studio-v2
- BASE_WP_ID: WP-1-Photo-Studio
- STUB_STATUS: SUPERSEDED
- SUPERSEDED_BY: WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1
- DEPRECATED_AT: 2026-05-16
- Created: 2026-01-11
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (currently Handshake_Master_Spec_v02.105.md)

## Deprecation
This stub must not be activated directly. Its skeleton surface, thumbnails, recipes, media viewer/DAM responsibility, and Loom preview-thumbnail overlap payload are preserved in `WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1` and its draft MT suite.

## Why this stub exists
This is an additive remediation stub for `WP-1-Photo-Studio`.

It is created because the prior packet failed revalidation and/or has audit gaps.

## Prior packet
- Prior WP_ID: `WP-1-Photo-Studio`
- Prior packet: `.GOV/task_packets/WP-1-Photo-Studio.md`

## Known gaps (Task Board summary)
- / FAIL: Skeleton surface, thumbnails, recipes. [READY FOR DEV]

## Activation checklist (before any coding)
1. In-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` Part 2.5.2).
2. USER_SIGNATURE.
3. Create `.GOV/refinements/WP-1-Photo-Studio-v2.md`.
4. Create official task packet via `just create-task-packet WP-1-Photo-Studio-v2`.
5. Update `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md` to point `WP-1-Photo-Studio` -> `WP-1-Photo-Studio-v2`.
6. Update `.GOV/roles_shared/TASK_BOARD.md` to move `WP-1-Photo-Studio-v2` out of STUB when activated.
