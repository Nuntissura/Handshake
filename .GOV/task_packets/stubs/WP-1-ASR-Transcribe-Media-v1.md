# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-ASR-Transcribe-Media-v1

## STUB_METADATA
- WP_ID: WP-1-ASR-Transcribe-Media-v1
- BASE_WP_ID: WP-1-ASR-Transcribe-Media
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: MEDIUM
- BUILD_ORDER_RISK_TIER: MEDIUM
- BUILD_ORDER_DEPENDS_ON: WP-1-Workflow-Engine, WP-1-MEX-v1.2-Runtime, WP-1-Artifact-System-Foundations
- BUILD_ORDER_BLOCKS: WP-1-Video-Archive-Loom-Integration
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: N/A (Enables transcript search and video archive utility)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md 6.2 Speech Recognition: ASR Subsystem
  - Handshake_Master_Spec_v02.139.md 10.12 Loom (DerivedContent + searchable transcripts)
  - Handshake_Master_Spec_v02.139.md 10.14 Media Downloader (captions sidecars + provenance)
  - Handshake_Master_Spec_v02.139.md 2.3.8 Shadow Workspace (Indexing/Search)
  - Handshake_Master_Spec_v02.139.md 11.1 Capabilities & Consent Model (`proc.exec`, `net.http`, `secrets.use`) (Normative)

## INTENT (DRAFT)
- What: Implement a local-first ASR transcription job for audio/video media, producing time-aligned transcripts that become searchable workspace entities.
- Why: Video archive value depends on searchability; captions are often missing or low quality, so an ASR fallback is required.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - JobKind + workflow execution:
    - Implement `asr_transcribe` as a Workflow Engine job (no UI bypass).
    - Extract audio from video deterministically (ffmpeg) as needed.
  - Outputs:
    - Persist transcript outputs as artifacts (e.g., JSON + readable text) with provenance and timing metadata.
    - Provide a path to attach transcript text to LoomBlocks (Document link or derived.full_text_index) to support Loom search.
  - Safety:
    - Capability gating for `proc.exec:*` and any model/tool invocation.
    - No cloud transcription by default (opt-in only).
  - Observability:
    - Flight Recorder events for job lifecycle, segment/chunk progress, and output artifact refs.
- OUT_OF_SCOPE:
  - Real-time/live captioning.
  - Accessibility-grade caption guarantees (WCAG compliance requires human review).

## ACCEPTANCE_CRITERIA (DRAFT)
- Given a video file with no captions, running ASR produces a transcript artifact with timestamps and makes it searchable via Loom search.
- Given a video with captions, the system can skip ASR and ingest captions into searchable text (optional fast-path).
- Jobs are cancellable, resumable where feasible, capability-gated, and leak-safe in logs/events.

## DEPENDENCIES / BLOCKERS (DRAFT)
- ffmpeg/ffprobe availability via Mechanical Tool Bus and provisioning.
- Selected local ASR engine (whisper.cpp / equivalent) and packaging strategy.
- Decision on canonical transcript storage format (WebVTT ingest vs JSON segments vs Markdown).

## RISKS / UNKNOWNs (DRAFT)
- Performance and memory for long videos; need chunking strategy and bounded resources.
- Transcript size/indexing cost; may require chunked indexing and retention policy.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-ASR-Transcribe-Media-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-ASR-Transcribe-Media-v1` (in `.GOV/task_packets/`).
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
