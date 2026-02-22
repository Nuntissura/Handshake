# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Video-Archive-Loom-Integration-v1

## STUB_METADATA
- WP_ID: WP-1-Video-Archive-Loom-Integration-v1
- BASE_WP_ID: WP-1-Video-Archive-Loom-Integration
- CREATED_AT: 2026-02-20T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: N/A (Operator request; unlocks Loom/Lens value for video assets)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.133.md 10.12 Loom (Heaper-style Library Surface) [ADD v02.130]
  - Handshake_Master_Spec_v02.133.md 2.2.1.14 LoomBlock Entity (Heaper-style Unit of Meaning) [ADD v02.130]
  - Handshake_Master_Spec_v02.133.md 2.3.7.1 Loom Relational Edges (Mentions, Tags, Backlinks) [ADD v02.130]
  - Handshake_Master_Spec_v02.133.md 6.2 Speech Recognition: ASR Subsystem (audio/video transcription into searchable text)
  - Handshake_Master_Spec_v02.133.md 2.3.5 Data Architecture: File-Tree Model (Sidecar Files)
  - Handshake_Master_Spec_v02.133.md 11.1 Capabilities & Consent Model (`fs.*`, `proc.exec`, `net.http` as needed) (Normative)

## INTENT (DRAFT)
- What: Turn archived/imported video files (from YouTube or elsewhere) into first-class Loom library objects with searchable transcripts, captions sidecars, and tag/mention organization that composes with Lens/Atelier.
- Why: Downloading is only half the problem; the archive must be browsable, searchable, and meaningfully organized across a large family library.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Loom-first modeling:
    - Each video Asset gets a LoomBlock wrapper (title, imported_at, source metadata).
    - Captions (`.vtt`) and transcript Documents attach as referenced entities (no duplication).
  - Transcript generation:
    - If captions are present: ingest captions as a timing sidecar and derive a readable transcript Document.
    - If captions are missing: offer local-first ASR job to produce transcript + timing sidecars (per §6.2).
  - Smart organization (user-safe):
    - Manual tags become RawContent `TAG` edges.
    - AI-suggested tags remain DerivedContent (`AI_SUGGESTED` edges) until user confirmation (LM-TAG-005).
  - Browsing + retrieval:
    - Loom views show videos with thumbnails/proxies and allow filtering by tags/date/source.
    - Lens queries can filter by tag facets and hit transcript/caption text.
  - Evidence + observability:
    - All ingestion/transcript/tagging jobs emit Flight Recorder events and are visible in Operator Consoles / Job History.
- OUT_OF_SCOPE:
  - Full video editor/timeline authoring UI (Director render pipelines are separate).
  - Cloud transcription by default (cloud is opt-in and capability-gated).

## ACCEPTANCE_CRITERIA (DRAFT)
- Importing a folder of videos + captions results in LoomBlocks with stable IDs, thumbnails, and attached captions/transcripts.
- Transcripts are searchable and link back to the source video (time offsets preserved).
- Tags/mentions work through LoomEdges; AI suggestions require explicit user confirmation before becoming Raw tags.
- A large library (100s+ videos) remains usable (batch jobs, progress, resume; no UI lockups).

## DEPENDENCIES / BLOCKERS (DRAFT)
- LoomBlock/LoomEdge foundations (Loom MVP).
- ASR ingestion pipeline + timing sidecar conventions.
- Asset preview/proxy generation primitives (thumbnails/keyframes).

## RISKS / UNKNOWNs (DRAFT)
- Transcript size + indexing cost; need chunking strategy for long videos.
- UX complexity: presenting captions vs ASR vs both; avoiding duplicate/conflicting transcripts.
- Privacy: ensure no default cloud sends; clear UI for capability/consent.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm Loom + ASR + tagging requirements are satisfied by referenced Master Spec sections.
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Video-Archive-Loom-Integration-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Video-Archive-Loom-Integration-v1` (in `.GOV/task_packets/`).
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

