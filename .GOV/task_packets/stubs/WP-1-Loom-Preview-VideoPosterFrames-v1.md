# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Loom-Preview-VideoPosterFrames-v1

## STUB_METADATA
- WP_ID: WP-1-Loom-Preview-VideoPosterFrames-v1
- BASE_WP_ID: WP-1-Loom-Preview-VideoPosterFrames
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Loom-MVP, WP-1-MEX-v1.2-Runtime
- BUILD_ORDER_BLOCKS: WP-1-Media-Downloader-Loom-Bridge, WP-1-Video-Archive-Loom-Integration
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Loom MVP / Cache-tiered browsing vertical slice
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md 10.12 Loom Section 6 Media & File Management: Cache-Tiered Asset Browsing (LM-CACHE-003, ThumbnailSpec)
  - Handshake_Master_Spec_v02.139.md 10.12 Loom Section 6.3 Thumbnail Generation Specification (video poster frames)
  - Handshake_Master_Spec_v02.139.md 10.10 Photo Studio (Proxy/Preview pipeline principles)
  - Handshake_Master_Spec_v02.139.md 11.1 Capabilities & Consent Model (`proc.exec`, `fs.*`) (Normative)

## INTENT (DRAFT)
- What: Support Tier-1 previews for video Assets by generating deterministic poster-frame thumbnails as a background mechanical job.
- Why: Loom browsing and media archive value collapse without thumbnails for videos; spec requires poster frames at Tier-1 preview.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Poster-frame thumbnail generation for video/* MIME:
    - Extract a representative frame (heuristic: first_non_black or configurable timestamp).
    - Resize to ThumbnailSpec max_dimension.
    - Encode to spec-preferred format.
    - Store as derived preview Asset and link it to LoomBlock (thumbnail_asset_id + preview_status).
  - Capability gating:
    - If using external tooling (ffmpeg/ffprobe), require and log the appropriate `proc.exec:*` capabilities.
  - Observability:
    - Emit Flight Recorder events for preview generation start/finish/failure with leak-safe payload.
- OUT_OF_SCOPE:
  - Tier-2 proxy generation.
  - Full audio waveform and document first-page previews (separate stubs if needed).

## ACCEPTANCE_CRITERIA (DRAFT)
- Importing/promoting a video into Loom results in a LoomBlock whose preview_status becomes generated and thumbnail is retrievable.
- Preview generation is non-blocking, resumable/idempotent, and emits Flight Recorder events.
- Unsupported/invalid video files fail with clear error_code without crashing the workflow engine.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Availability of ffmpeg/ffprobe tooling in the Mechanical Tool Bus provisioning.
- Final decision on thumbnail format (WebP vs PNG) and max dimension (512 vs current defaults).

## RISKS / UNKNOWNs (DRAFT)
- Determinism: frame selection heuristics may vary across ffmpeg versions; may need pinned versions and fixed flags.
- Performance: large videos; extracting a frame should be bounded and cancellable.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Loom-Preview-VideoPosterFrames-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Loom-Preview-VideoPosterFrames-v1` (in `.GOV/task_packets/`).
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
