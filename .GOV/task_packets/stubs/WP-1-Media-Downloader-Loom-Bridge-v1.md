# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Media-Downloader-Loom-Bridge-v1

## STUB_METADATA
- WP_ID: WP-1-Media-Downloader-Loom-Bridge-v1
- BASE_WP_ID: WP-1-Media-Downloader-Loom-Bridge
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Media-Downloader, WP-1-Loom-MVP, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-Video-Archive-Loom-Integration
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: N/A (Bridges existing surfaces; unlocks Loom value for archived media)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md 10.14 Media Downloader (Unified Web Media Archiving Surface) [ADD v02.134]
  - Handshake_Master_Spec_v02.139.md 10.14.6 Output routing (Normative)
  - Handshake_Master_Spec_v02.139.md 10.14.7 YouTube archive requirements (Normative)
  - Handshake_Master_Spec_v02.139.md 10.14.8 Generic video downloader requirements (Normative)
  - Handshake_Master_Spec_v02.139.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]
  - Handshake_Master_Spec_v02.139.md 10.12 Section 6 Media & File Management: Cache-Tiered Asset Browsing (LM-MEDIA-001, LM-CACHE-003)
  - Handshake_Master_Spec_v02.139.md 2.2.3.1 Asset Entity + ProxySettings
  - Handshake_Master_Spec_v02.139.md 2.3.5 Data Architecture: File-Tree Model (Sidecar Files)
  - Handshake_Master_Spec_v02.139.md 2.3.8 Shadow Workspace (Indexing/Search)
  - Handshake_Master_Spec_v02.139.md 11.1 Capabilities & Consent Model (`fs.*`, `proc.exec`, `net.http`, `secrets.use`) (Normative)

## INTENT (DRAFT)
- What: Make Media Downloader outputs promotable into Loom as LoomBlocks (with stable IDs, dedup, previews, and searchable text when captions/transcripts exist).
- Why: Media Downloader creates a local archive, but without a Loom wrapper the library is not browsable/searchable in the Loom surface and does not compose with tags/mentions/backlinks.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Bridge primitive:
    - Given a completed Media Downloader item (artifact_handles + materialized_paths), create or update:
      - Asset record(s) in the workspace graph (content-hash stable).
      - LoomBlock wrapper for the primary media file (video/audio/image) with imported_at and source metadata.
  - Dedup:
    - Repeated promotion MUST be idempotent and deduplicate by content_hash (workspace-scoped).
  - Sidecars:
    - Captions (`.vtt`) + captions metadata (`captions.metadata.json`) are preserved as sidecars and linked to the LoomBlock (no duplication).
    - `info.json` (when present) is stored and linked as provenance/tool-output.
  - Preview:
    - Promotion SHOULD enqueue Tier-1 preview generation for the LoomBlock (thumbnail/poster frame).
  - Search:
    - When a caption track exists, ingest captions into a searchable text layer (either a Document linked to LoomBlock or LoomBlock.derived.full_text_index).
  - Evidence:
    - Emit Flight Recorder events for: promotion requested, dedup hit, assets created/linked, captions ingested, preview queued/completed.
- OUT_OF_SCOPE:
  - Full Loom UI delivery (grid/list, detail view) unless minimally needed to validate the bridge.
  - Cloud transcription by default (opt-in and capability-gated only).

## ACCEPTANCE_CRITERIA (DRAFT)
- After a YouTube archive job completes, a user can promote the downloaded video into Loom:
  - LoomBlock exists, points at the media Asset, and is dedup-stable across repeated promotions.
  - Captions sidecars (when present) are linked and preserved.
  - Loom search can find text from captions/transcript for that video.
  - Preview generation is queued and results in a thumbnail/poster frame.
- All operations are capability-gated and emit Flight Recorder events without secret leakage.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Loom MVP foundations (LoomBlock/LoomEdge + import/dedup).
- Preview generation for video poster frames (if not already supported).
- A representation decision: unify Media Downloader "artifact" files with Loom "Asset" entities or implement a stable, lossless promotion path from artifacts -> assets.

## RISKS / UNKNOWNs (DRAFT)
- Data model alignment: risk of parallel "artifact" vs "asset" stores creating duplication or confusing identity semantics.
- Large file handling: avoid base64-in-JSON or other non-streaming imports for multi-GB videos.
- Caption variability: multiple languages/tracks, auto-captions quality, and track selection UX.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Media-Downloader-Loom-Bridge-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Media-Downloader-Loom-Bridge-v1` (in `.GOV/task_packets/`).
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
