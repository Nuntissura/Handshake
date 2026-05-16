---
file_id: handshake-stub-preservation-map
file_kind: ckc_atelier_lens_consolidation_reference
updated_at: 2026-05-16
---

# Handshake Stub Preservation Map

This map inventories Atelier, Lens, Photo Studio, Studio runtime, Stage/media, ASR, Loom/video, and closely adjacent support stubs that must be preserved or considered during CKC consolidation. It is a preservation document, not activation authority. Stubs remain non-executable until the normal refinement, signature, packet creation, task board, and traceability workflows promote them.

Read sources:

- `.GOV/task_packets/stubs`
- `.GOV/roles_shared/records/TASK_BOARD.md`
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
- `.GOV/roles_shared/records/BUILD_ORDER.md`
- `.GOV/spec/master-spec-v02.185`

## Major Findings

- CKC should not drop the older Atelier, Lens, Photo Studio, Studio, Stage, ASR, Loom, media downloader, screenshot, or visual-debugging plans. Treat them as source material, historical evidence, or inherited platform dependencies depending on lifecycle state.
- Several source stubs are lifecycle-misaligned across surfaces. `WP-1-Atelier-Collaboration-Panel-v1`, `WP-1-Lens-ViewMode-v1`, `WP-1-Loom-MVP-v1`, `WP-1-Media-Downloader-v1`, and `WP-1-Loom-Storage-Portability-v4` still exist under stubs, while task board or registry state says the active work is validated, done, superseded, or moved to a packet path.
- `WP-1-Atelier-Lens-v2` and `WP-1-Photo-Studio-v2` have sparse machine contracts and only meaningful scope in the Markdown projection. Preserve their known gaps directly: Atelier role claiming, SceneState, ConflictSet; Photo Studio skeleton surface, thumbnails, recipes.
- Stage/media and Stage/ASR stubs are high-value, high-risk backend contracts. They should feed the CKC foundation runway before UI polish, because they define manifest, provenance, source hash, timing anchor, and portable evidence semantics.
- Loom/video/media downloader stubs form one coherent runway: media archive -> portable artifact handles -> LoomBlock promotion -> poster frames -> transcript or caption indexing -> Lens query reuse.
- Visual validation and product screenshot stubs are already folded into Kernel002, but CKC still needs their goals as inherited validation requirements: screenshot evidence, panel capture, visual diff artifacts, and GUI regression routing.
- Stale references exist in some activation checklists that point to `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md` and `.GOV/roles_shared/TASK_BOARD.md`; current read sources use `.GOV/roles_shared/records/...`. Future registry updates should normalize these paths.

## Inventory

### WP-1-Atelier-Lens-v2

- Source paths: `.GOV/task_packets/stubs/WP-1-Atelier-Lens-v2.md`, `.GOV/task_packets/stubs/WP-1-Atelier-Lens-v2.contract.json`
- Board and registry state: stub backlog, not activated; base `WP-1-Atelier-Lens` is superseded.
- Original intent: additive remediation for failed or gapped `WP-1-Atelier-Lens`.
- Preserved scope: restore or replace the missing Atelier/Lens behavior around role claiming, SceneState, and ConflictSet.
- Acceptance preserved: no acceptance text in the sparse contract; infer activation needs from the known gap list and the prior packet remediation checklist.
- Dependencies preserved: no explicit build-order dependencies in the stub; `Studio-Runtime-Visibility` depends on this base WP.
- Risks preserved: sparse contract creates preservation risk; prior packet paths were not read in this task, so CKC must not pretend full original behavior was recovered.
- CKC handling: source material for CKC Atelier/Lens runtime WP; promote via a new CKC runway stub instead of editing this stub.

### WP-1-Photo-Studio-v2

- Source paths: `.GOV/task_packets/stubs/WP-1-Photo-Studio-v2.md`, `.GOV/task_packets/stubs/WP-1-Photo-Studio-v2.contract.json`
- Board and registry state: stub backlog, not activated; base `WP-1-Photo-Studio` is superseded.
- Original intent: additive remediation for failed or gapped `WP-1-Photo-Studio`.
- Preserved scope: skeleton surface, thumbnails, and recipes remain planned work.
- Acceptance preserved: no acceptance text in the sparse contract; acceptance must be re-derived in the CKC refinement from skeleton visibility, thumbnail generation or display, and recipe persistence/use.
- Dependencies preserved: no explicit build-order dependencies in the stub; `Studio-Runtime-Visibility` depends on this base WP.
- Risks preserved: sparse contract, possible overlap with Loom preview thumbnails and media poster frames.
- CKC handling: source material for CKC Studio surface WP; keep Photo Studio-specific recipes separate from Loom/video archive ingestion unless a shared preview service is explicitly defined.

### WP-1-Atelier-Collaboration-Panel-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Atelier-Collaboration-Panel-v1.md`, `.GOV/task_packets/stubs/WP-1-Atelier-Collaboration-Panel-v1.contract.json`
- Board and registry state: validated/done as an active packet; stub is retained as source evidence.
- Original intent: implement a selection-scoped Atelier Collaboration Panel in editor surfaces.
- Preserved scope: role suggestions over current selection, multi-suggestion review, strict range-bounded patching for Monaco/Docs, provenance logging, and validator rejection of out-of-selection patches.
- Acceptance preserved: applying suggestions must never change text outside the selected span; validators reject out-of-range patches; applied patches have provenance and evidence refs visible in audit surfaces.
- Dependencies preserved: none explicit in contract.
- Risks preserved: editor range semantics can cause off-by-one errors; boundary normalization can become a loophole.
- CKC handling: remain a separate validated source pattern; CKC should reuse selection-scoped patch law and provenance, not supersede it wholesale.

### WP-1-Lens-Extraction-Tier-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Lens-Extraction-Tier-v1.md`, `.GOV/task_packets/stubs/WP-1-Lens-Extraction-Tier-v1.contract.json`
- Board and registry state: stub backlog, not activated.
- Original intent: implement `LensExtractionTier` as a first-class runtime and planning input.
- Preserved scope: Tier0/Tier1/Tier2 model, Tier1 default, explicit override for Tier0/Tier2, trace requested versus effective tier, validation against invalid tier use.
- Acceptance preserved: Tier1 defaults when unspecified; Tier0/Tier2 override intent is trace-visible; validators enforce defaults and reject misuse.
- Dependencies preserved: none explicit in contract.
- Risks preserved: confusion with `content_tier`; implicit Tier2 creep through heuristics.
- CKC handling: source material for CKC Lens runtime controls. Keep extraction tier orthogonal to safety/view projection.

### WP-1-Lens-ViewMode-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Lens-ViewMode-v1.md`, `.GOV/task_packets/stubs/WP-1-Lens-ViewMode-v1.contract.json`
- Board and registry state: validated/done as an active packet; stub retained as source evidence.
- Original intent: implement `ViewMode` UI and enforcement for Lens outputs.
- Preserved scope: NSFW default, explicit SFW toggle, hard-drop SFW projection, immutable raw and derived artifacts, trace-visible ViewMode metadata filter.
- Acceptance preserved: SFW ViewMode never shows non-sfw items; toggling ViewMode does not mutate stored artifacts; QueryPlan or RetrievalTrace records ViewMode as a filter.
- Dependencies preserved: none explicit in contract.
- Risks preserved: inconsistent enforcement across surfaces; accidental mutation of stored artifacts.
- CKC handling: remain separate validated source law. CKC should consume the projection rule and test that media review surfaces do not mutate raw evidence.

### WP-1-Stage-Media-Artifact-Portability-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Stage-Media-Artifact-Portability-v1.md`, `.GOV/task_packets/stubs/WP-1-Stage-Media-Artifact-Portability-v1.contract.json`
- Board and registry state: stub backlog; Build Order rank 1, value high, risk high, blocked.
- Original intent: unify Stage capture/import sessions and Media Downloader outputs under portable artifact manifest, bundle-index, and retention semantics.
- Preserved scope: shared artifact portability contract across Stage session records, media capture sessions, debug bundle export, and storage portability rules; session/auth/materialization outputs as bounded export anchors.
- Acceptance preserved: Stage and Media Downloader are linked through portable artifact and retention contracts; later packets reuse one bounded manifest contract; storage swaps do not redefine evidence semantics.
- Dependencies preserved: `WP-1-Handshake-Stage-MVP`, `WP-1-Media-Downloader`, `WP-1-Artifact-System-Foundations`, `WP-1-Storage-Trait-Purity`.
- Unlocks preserved: `WP-1-Media-Downloader-Loom-Bridge`, `WP-1-Video-Archive-Loom-Integration`.
- Risks preserved: Stage and media artifacts may fork incompatible manifests; capture-session provenance may be omitted from general portability rules.
- CKC handling: become source material for CKC foundation WP. This should precede downstream media-to-Loom consolidation.

### WP-1-Stage-ASR-Transcript-Lineage-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Stage-ASR-Transcript-Lineage-v1.md`, `.GOV/task_packets/stubs/WP-1-Stage-ASR-Transcript-Lineage-v1.contract.json`
- Board and registry state: stub backlog; Build Order rank 9, value high, risk high, blocked.
- Original intent: define the backend lineage from Stage-captured/imported media to governed ASR transcript artifacts.
- Preserved scope: source media artifact -> ASR input -> transcript artifact -> searchable consumer; stable source hash, media probe facts, capture/session provenance, timing anchors, recorder-visible events, storage-portable transcript linkage.
- Acceptance preserved: Stage media and ASR transcripts use a single backend lineage contract; later packets reuse source hash, timing anchor, and provenance contracts across Stage, ASR, Loom, and archive flows.
- Dependencies preserved: `WP-1-Handshake-Stage-MVP`, `WP-1-ASR-Transcribe-Media`, `WP-1-Media-Downloader`, `WP-1-Artifact-System-Foundations`.
- Unlocks preserved: `WP-1-Video-Archive-Loom-Integration`.
- Risks preserved: separate identity fields across Stage/media/ASR; transcript text survives while timing anchors or provenance are lost.
- CKC handling: source material for CKC foundation WP and CKC media intelligence WP; do not bury it as an implementation detail.

### WP-1-Studio-Runtime-Visibility-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Studio-Runtime-Visibility-v1.md`, `.GOV/task_packets/stubs/WP-1-Studio-Runtime-Visibility-v1.contract.json`
- Board and registry state: stub backlog; Build Order rank 23, value high, risk medium, blocked.
- Original intent: make Studio and Design Studio surfaces explicit runtime citizens.
- Preserved scope: runtime mappings for Canvas/Excalidraw, Lens/Atelier collaboration panel, Studio jobs, workflow nodes, tool surfaces, DCC/operator projection, Flight Recorder linkage, Locus/task-board/WP linkage, and PostgreSQL-only state.
- Acceptance preserved: Studio-adjacent surfaces have explicit job/workflow/tool mappings; Studio activity is visible in Command Center and Flight Recorder; Locus correlates Studio work; storage posture is specified.
- Dependencies preserved: `WP-1-Photo-Studio`, `WP-1-Atelier-Lens`, `WP-1-Dev-Command-Center-MVP`, `WP-1-Locus-Work-Tracking-System-Phase1`.
- Risks preserved: fragmented UI panels, hidden side effects, and any accidental SQLite reintroduction.
- CKC handling: source material for CKC Atelier/Lens/Studio runtime WP.

### WP-1-ASR-Transcribe-Media-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-ASR-Transcribe-Media-v1.md`, `.GOV/task_packets/stubs/WP-1-ASR-Transcribe-Media-v1.contract.json`
- Board and registry state: stub backlog; Build Order rank 36, value medium, risk medium.
- Original intent: implement local-first ASR transcription for audio/video media.
- Preserved scope: `asr_transcribe` Workflow Engine job, deterministic audio extraction through ffmpeg, transcript artifacts in JSON and readable text, timing metadata, attachment to LoomBlocks or full-text index, capability gating, no cloud transcription by default, Flight Recorder progress/events.
- Acceptance preserved: videos without captions produce timestamped transcript artifacts searchable through Loom; captioned videos can skip ASR and ingest captions; jobs are cancellable, resumable where feasible, capability-gated, and leak-safe.
- Dependencies preserved: `WP-1-Workflow-Engine`, `WP-1-MEX-v1.2-Runtime`, `WP-1-Artifact-System-Foundations`.
- Unlocks preserved: `WP-1-Video-Archive-Loom-Integration`.
- Risks preserved: long-video performance and memory; transcript size and indexing cost.
- CKC handling: source material for CKC media intelligence WP; keep local-first/cloud-opt-in posture.

### WP-1-Video-Archive-Loom-Integration-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Video-Archive-Loom-Integration-v1.md`, `.GOV/task_packets/stubs/WP-1-Video-Archive-Loom-Integration-v1.contract.json`
- Board and registry state: stub backlog; Build Order rank 10, value high, risk high, blocked.
- Original intent: turn archived/imported video files into Loom library objects with searchable transcripts, captions sidecars, and tag/mention organization that composes with Lens/Atelier.
- Preserved scope: LoomBlock wrappers, captions and transcript documents as referenced entities, captions ingest fast path, local-first ASR fallback, manual RawContent tags, AI-suggested DerivedContent tags pending confirmation, Loom views with thumbnails/proxies, transcript and caption Lens retrieval, Flight Recorder events.
- Acceptance preserved: importing video folders creates stable LoomBlocks with thumbnails and transcripts; transcripts are searchable and time-offset linked; tags/mentions use LoomEdges; large libraries remain usable with batch progress and resume.
- Dependencies preserved: `WP-1-Media-Downloader`, `WP-1-Loom-MVP`, `WP-1-Media-Downloader-Loom-Bridge`, `WP-1-Loom-Preview-VideoPosterFrames`, `WP-1-ASR-Transcribe-Media`.
- Risks preserved: transcript indexing cost, captions versus ASR duplicates, privacy/cloud-send confusion.
- CKC handling: source material for CKC media intelligence WP; do not collapse into generic CKC UI until backend lineage and storage contracts are explicit.

### WP-1-Loom-MVP-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Loom-MVP-v1.md`, `.GOV/task_packets/stubs/WP-1-Loom-MVP-v1.contract.json`
- Board and registry state: validated/done as active packet; stub retained as origin evidence.
- Original intent: deliver the Phase 1 Loom MVP local-first library surface.
- Preserved scope: LoomBlocks, LoomEdges, stable UUID identity, @mentions, #tags, All/Unlinked/Sorted/Pins views, backlinks panel, file import, SHA-256 dedup, Tier-1 previews, basic search, FR-EVT-LOOM events.
- Acceptance preserved: imports create LoomBlocks; dedup prevents duplicates; mentions/tags create stable LoomEdges; backlinks update; Tier-1 search works; FR-EVT-LOOM events surface in operator history.
- Dependencies preserved: none explicit in contract.
- Risks preserved: anchor drift, dedup identity errors, preview generation degrading UI responsiveness.
- CKC handling: source baseline and dependency; CKC should extend, not replace, Loom core identity semantics.

### WP-1-Loom-Storage-Portability-v4

- Source paths: `.GOV/task_packets/stubs/WP-1-Loom-Storage-Portability-v4.md`, `.GOV/task_packets/stubs/WP-1-Loom-Storage-Portability-v4.contract.json`
- Board and registry state: task board says validated; registry row is internally mixed, saying active v4 and Stub Backlog. Historical v1/v2/v3 are superseded or failed historical.
- Original intent: re-open Loom portability as a narrow remediation/proof pass separating real portability evidence from narrative closure.
- Preserved scope: revalidate current-spec Loom portability clauses, avoid speculative churn, and preserve PostgreSQL-only evidence for graph traversal, directional edges, metrics recomputation, and source-anchor durability. Older dual-backend intent is superseded.
- Acceptance preserved: remediation ties to current clauses and evidence; concrete defects are proven by PostgreSQL-only checks; if no defect remains, refinement narrows scope rather than inventing churn.
- Dependencies preserved: `WP-1-Loom-MVP`, `WP-1-Storage-Abstraction-Layer`, `WP-1-Artifact-System-Foundations`.
- Unlocks preserved: `WP-1-Media-Downloader-Loom-Bridge`, `WP-1-Video-Archive-Loom-Integration`.
- Risks preserved: old v3 mixed concrete storage work with broad closure claims; broad storage proof can expand too far.
- CKC handling: remain separate validated or proof baseline; CKC should depend on its active portability evidence and record the registry inconsistency for later cleanup.

### WP-1-Loom-Preview-VideoPosterFrames-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Loom-Preview-VideoPosterFrames-v1.md`, `.GOV/task_packets/stubs/WP-1-Loom-Preview-VideoPosterFrames-v1.contract.json`
- Board and registry state: stub backlog; Build Order rank 20, value high, risk medium.
- Original intent: support Tier-1 previews for video assets by generating deterministic poster-frame thumbnails as a background mechanical job.
- Preserved scope: extract representative frame, resize to ThumbnailSpec, encode to preferred format, store as derived preview asset, link to LoomBlock, capability-gate ffmpeg/ffprobe, emit Flight Recorder events.
- Acceptance preserved: video imports/promotions produce retrievable thumbnails and generated preview status; generation is non-blocking, resumable, idempotent; invalid files fail with clear error codes.
- Dependencies preserved: `WP-1-Loom-MVP`, `WP-1-MEX-v1.2-Runtime`.
- Unlocks preserved: `WP-1-Media-Downloader-Loom-Bridge`, `WP-1-Video-Archive-Loom-Integration`.
- Risks preserved: ffmpeg-version determinism; large-video performance and cancellation.
- CKC handling: source material for CKC media intelligence WP and shared preview service.

### WP-1-Media-Downloader-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Media-Downloader-v1.md`, `.GOV/task_packets/stubs/WP-1-Media-Downloader-v1.contract.json`
- Board and registry state: task board says v1 is superseded and v2 is validated/done; registry active path is `.GOV/task_packets/WP-1-Media-Downloader-v2.md`.
- Original intent: batch archive web media into local-first resumable ingest jobs with capability gating and evidence logging.
- Preserved scope: one Media Downloader surface for YouTube, Instagram, forum/blog images, and generic video URLs; unified queue/progress; user-configurable output root; public/account modes; captions sidecars; forum full-res image crawling; resumable queue; rate limiting; deterministic naming and sidecars; optional Director transcode; Bronze/Raw asset materialization; Flight Recorder events; capability gating.
- Acceptance preserved: YouTube/Instagram/forum/generic media archive resumes and avoids duplicates; captions saved as VTT; transcode deterministic; network/exec actions gated; UI shows per-item and batch progress.
- Dependencies preserved: none explicit in v1 contract.
- Risks preserved: platform policy changes, fragile parsing, large storage, partial downloads, caption variability, forum variance.
- CKC handling: treat v1 as preserved historical source and v2 as the active dependency from registry; CKC should not revive v1 except to preserve media-mode requirements that might not be visible in v2.

### WP-1-Media-Downloader-Loom-Bridge-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Media-Downloader-Loom-Bridge-v1.md`, `.GOV/task_packets/stubs/WP-1-Media-Downloader-Loom-Bridge-v1.contract.json`
- Board and registry state: stub backlog; Build Order rank 30, value high, risk medium.
- Original intent: make Media Downloader outputs promotable into Loom as LoomBlocks.
- Preserved scope: bridge completed downloader artifacts/materialized paths into workspace Asset records and LoomBlock wrappers; dedup by content hash; preserve captions and metadata sidecars; enqueue previews; ingest captions into searchable layer; emit Flight Recorder promotion, dedup, linking, captions, and preview events.
- Acceptance preserved: completed YouTube archive promotes into a dedup-stable LoomBlock with media Asset, captions sidecars, searchable caption text, and queued/generated preview; operations are capability-gated and leak-safe.
- Dependencies preserved: `WP-1-Media-Downloader`, `WP-1-Loom-MVP`, `WP-1-Artifact-System-Foundations`.
- Unlocks preserved: `WP-1-Video-Archive-Loom-Integration`.
- Risks preserved: artifact versus asset store duplication; large-file non-streaming imports; caption language/track variability.
- CKC handling: source material for CKC media intelligence WP; should sit after artifact portability and before archive UI consolidation.

### WP-1-Product-Screenshot-Visual-Validation-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Product-Screenshot-Visual-Validation-v1.md`, `.GOV/task_packets/stubs/WP-1-Product-Screenshot-Visual-Validation-v1.contract.json`
- Board and registry state: superseded, folded into Kernel002.
- Original intent: build product-integrated screenshot capture for full app window, individual panels, and module-level views.
- Preserved scope: programmatic full-window, panel, and module capture; governed artifact storage; CLI/API trigger for coder/validator sessions; screenshot metadata; Tauri/webview/native integration.
- Acceptance preserved: governed coder and validator sessions can trigger and inspect screenshots; screenshots are stored with metadata; capture works on Tauri + React; usable from cloud and local model sessions.
- Dependencies preserved: none explicit.
- Risks preserved: panel granularity may need DOM capture; headless/mock UI may differ; screenshot file size requires retention policy; native versus DOM capture uncertainty on Windows 11.
- CKC handling: remain superseded platform dependency. CKC should inherit its requirements through Kernel002/Kernel003 visual evidence, not reimplement capture.

### WP-1-Visual-Debugging-Loop-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Visual-Debugging-Loop-v1.md`, `.GOV/task_packets/stubs/WP-1-Visual-Debugging-Loop-v1.contract.json`
- Board and registry state: superseded, folded into Kernel002.
- Original intent: implement generate-capture-compare-fix visual debugging loop for GUI work packets.
- Preserved scope: post-commit screenshot capture trigger for GUI-bearing WPs, visual comparison against baseline, visual diff artifacts, validator evidence routing, threshold configuration, Tauri app test mode.
- Acceptance preserved: GUI WPs trigger captures; diffs are stored with WP and commit metadata; validators receive visual evidence; threshold breaches create STEER feedback.
- Dependencies preserved: `WP-1-Product-Screenshot-Visual-Validation`.
- Risks preserved: pixel false positives, headless rendering mismatch, threshold tuning noise.
- CKC handling: inherited validation requirement. CKC GUI microtasks should reference Kernel visual evidence rather than create a separate visual loop.

### WP-1-Calendar-Lens-v3

- Source paths: `.GOV/task_packets/stubs/WP-1-Calendar-Lens-v3.md`, `.GOV/task_packets/stubs/WP-1-Calendar-Lens-v3.contract.json`
- Board and registry state: stub backlog; v2 superseded.
- Original intent: implement Calendar Lens as first-class UI and API workflow.
- Preserved scope: day/week/month or staged baseline view, source/date filters, title search, backend CalendarSource/CalendarEvent query, stable IDs, provenance, optional read-only overlay into timeline/Flight Recorder.
- Acceptance preserved: user can open Calendar Lens, see events for a selected window, add at least one CalendarSource, filter by source, and reload events without drift.
- Dependencies preserved: `WP-1-Calendar-Storage`.
- Risks preserved: timezone/DST; event privacy and accidental indexing.
- CKC handling: not media-specific, but preserve as Lens pattern material for shared query/view contracts.

### WP-1-Artifact-System-Foundations-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Artifact-System-Foundations-v1.md`, `.GOV/task_packets/stubs/WP-1-Artifact-System-Foundations-v1.contract.json`
- Board and registry state: supporting stub/dependency.
- Original intent: ensure artifact system foundations across exports and jobs.
- Preserved scope: artifact store bootstrap, manifests, SHA-256, single atomic Materialize API, retention/pinning/GC, no random filesystem side effects.
- Acceptance preserved: all export paths use one atomic materialize implementation; artifacts and bundles have stable manifests and hashes; retention/GC never deletes pinned items and emits a visible report.
- Dependencies preserved: coordinates with Debug Bundle and Workspace Bundle.
- Risks preserved: competing materialize implementations; retention/GC data-loss bugs.
- CKC handling: foundational dependency for every CKC media, transcript, screenshot, and preview artifact path.

### WP-1-Structured-Collaboration-Artifact-Family-v1

- Source paths: `.GOV/task_packets/stubs/WP-1-Structured-Collaboration-Artifact-Family-v1.md`, `.GOV/task_packets/stubs/WP-1-Structured-Collaboration-Artifact-Family-v1.contract.json`
- Board and registry state: stub backlog/supporting dependency.
- Original intent: define canonical structured collaboration artifacts for work packets, microtasks, task board projections, and role mailbox exports.
- Preserved scope: packet, summary, index, and thread JSON/JSONL artifacts; versioned schemas; project-agnostic envelope; profile extensions; compact summaries; validation/migration from Markdown-first records.
- Acceptance preserved: versioned structured file family; compact summaries for small local models; profile extension separation; migration guidance from Markdown authority to mirrors/sidecars.
- Dependencies preserved: `WP-1-Locus-Phase1-Integration-Occupancy`, `WP-1-Role-Mailbox`, `WP-1-Micro-Task-Executor`.
- Risks preserved: overfitting to repository-centric work; summary drift; premature migration complexity.
- CKC handling: registry and microtask substrate for CKC runway packets.

## Conflicts, Stale References, And Supersession Risks

1. Sparse contract versus meaningful Markdown for `Atelier-Lens-v2` and `Photo-Studio-v2`.
   - Risk: CKC could drop original gap intent because the machine contract is empty.
   - Layered handling: preserve the Markdown gap list as source material, then create fresh CKC refinements with explicit scope and acceptance.

2. Validated work still has stub artifacts in the stubs directory.
   - Affected: `Atelier-Collaboration-Panel-v1`, `Lens-ViewMode-v1`, `Loom-MVP-v1`, `Media-Downloader-v1/v2`, `Loom-Storage-Portability-v4`.
   - Risk: CKC could reopen completed work or treat historical stubs as executable.
   - Layered handling: keep validated work as baseline contracts and source evidence; CKC only adds deltas.

3. Superseded visual stubs are still needed as validation requirements.
   - Affected: `Product-Screenshot-Visual-Validation-v1`, `Visual-Debugging-Loop-v1`.
   - Risk: CKC drops screenshot and visual regression evidence because the stubs are superseded.
   - Layered handling: consume them through Kernel002/Kernel003 validation requirements; do not reimplement unless Kernel coverage is missing.

4. Stage/media build order is high priority but blocked by downstream-facing packets.
   - Risk: archive/bridge work invents manifest and lineage semantics before the backend portability contract is settled.
   - Layered handling: CKC foundation WP defines artifact and lineage contracts first, then media intelligence WP consumes them.

5. ViewMode projection can conflict with raw media evidence preservation.
   - Risk: SFW presentation could mutate or hide underlying evidence in a way that breaks replay or audit.
   - Layered handling: ViewMode remains a projection filter only; raw artifacts and evidence refs stay immutable and trace-visible.

6. Media Downloader v1 is superseded by v2, but v1 contains detailed source-mode requirements.
   - Risk: v1 requirements for Instagram, forum image crawling, captions, progress, and output layout are lost if only active v2 path is followed.
   - Layered handling: use v2 as active dependency, but copy preserved v1 requirements into CKC source-material notes before scoping archive consolidation.

7. Traceability path references drift.
   - Risk: activation checklists point at `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md` and `.GOV/roles_shared/TASK_BOARD.md`, while current records live under `.GOV/roles_shared/records/`.
   - Layered handling: later registry cleanup should normalize current record paths without editing historical stubs.

## Consolidation Strategy

- Source material for new CKC runway stubs: `WP-1-Atelier-Lens-v2`, `WP-1-Photo-Studio-v2`, `WP-1-Lens-Extraction-Tier-v1`, `WP-1-Stage-Media-Artifact-Portability-v1`, `WP-1-Stage-ASR-Transcript-Lineage-v1`, `WP-1-Studio-Runtime-Visibility-v1`, `WP-1-ASR-Transcribe-Media-v1`, `WP-1-Video-Archive-Loom-Integration-v1`, `WP-1-Loom-Preview-VideoPosterFrames-v1`, `WP-1-Media-Downloader-Loom-Bridge-v1`.
- Separate validated baselines to reuse: `WP-1-Atelier-Collaboration-Panel-v1`, `WP-1-Lens-ViewMode-v1`, `WP-1-Loom-MVP-v1`, `WP-1-Media-Downloader-v2` as referenced by registry, and `WP-1-Loom-Storage-Portability-v4` after its registry state is clarified.
- Historical source material to preserve but not reactivate directly: `WP-1-Media-Downloader-v1`, `WP-1-Loom-Storage-Portability-v1/v2/v3`, original `WP-1-Atelier-Lens`, original `WP-1-Photo-Studio`, and `WP-1-Calendar-Lens-v2`.
- Superseded platform requirements to inherit: `WP-1-Product-Screenshot-Visual-Validation-v1` and `WP-1-Visual-Debugging-Loop-v1`, through Kernel002/Kernel003 validation surfaces.
- Adjacent but separate Lens pattern source: `WP-1-Calendar-Lens-v3`; keep it separate from CKC media work but reuse query/view model ideas.
- Foundational dependencies that should remain separate: `WP-1-Artifact-System-Foundations-v1`, `WP-1-Structured-Collaboration-Artifact-Family-v1`, and Postgres structured artifact parity.

Suggested new CKC runway stubs:

1. `WP-CKC-001-Media-Lineage-Artifact-Foundation-v1`
   - Consolidates Stage/media portability, Stage/ASR transcript lineage, artifact foundations, source hash/timing anchor contracts, retention, and traceability.

2. `WP-CKC-002-Atelier-Lens-Studio-Runtime-v1`
   - Consolidates Atelier-Lens remediation, Lens extraction tier, ViewMode reuse, Studio runtime visibility, Photo Studio skeleton/thumbnails/recipes, and selection-scoped collaboration reuse.

3. `WP-CKC-003-Loom-Archive-Media-Intelligence-v1`
   - Consolidates Media Downloader active output, Media Downloader v1 preserved source modes, Loom bridge, video poster frames, ASR transcription, video archive/Loom integration, search/tag/caption flows, and inherited visual validation.

## Later Traceability Registry Changes Needed

- Add CKC base WP entries and mark them as active runway stubs once the operator approves them.
- Add `source_material` links from each CKC WP to every preserved source stub named above.
- Add `supersedes_by_consolidation` only after the new CKC packet is signed; until then use `considered_by_ckc` or equivalent non-authoritative linkage.
- Normalize current registry/task-board paths to `.GOV/roles_shared/records/...` in new packet checklists.
- Clarify `WP-1-Loom-Storage-Portability-v4` state because task board says validated while the registry status text says Stub Backlog and active v4.
- Record validated baseline dependencies separately from superseded historical evidence so agents do not reopen done work.
- Add explicit "do not edit historical stub" notes for superseded and validated source stubs.

## Massive WP Plan With Microtask Buckets

### WP-CKC-001-Media-Lineage-Artifact-Foundation-v1

1. CKC1-MT001 source map verification for all Stage/media/ASR/artifact stubs.
2. CKC1-MT002 current registry and task-board status reconciliation report.
3. CKC1-MT003 artifact identity contract for Stage capture/import sessions.
4. CKC1-MT004 media downloader output handle contract.
5. CKC1-MT005 portable manifest schema for media capture sessions.
6. CKC1-MT006 bundle index schema for media and transcript artifacts.
7. CKC1-MT007 retention and pinning policy mapping for media artifacts.
8. CKC1-MT008 materialize-only write path rule for CKC media outputs.
9. CKC1-MT009 stable source hash contract for video/audio/image sources.
10. CKC1-MT010 media probe fact schema with ffprobe provenance.
11. CKC1-MT011 Stage capture provenance fields.
12. CKC1-MT012 Media Downloader provenance fields.
13. CKC1-MT013 ASR job input identity schema.
14. CKC1-MT014 transcript artifact schema with timing anchors.
15. CKC1-MT015 caption sidecar schema with source track metadata.
16. CKC1-MT016 source media to transcript lineage edge.
17. CKC1-MT017 transcript to searchable consumer lineage edge.
18. CKC1-MT018 Flight Recorder event family plan for capture/import/transcribe.
19. CKC1-MT019 debug bundle export anchors for media sessions.
20. CKC1-MT020 storage backend portability conformance matrix.
21. CKC1-MT021 redaction/exportability defaults for media artifacts.
22. CKC1-MT022 deterministic replay checklist for media lineage.
23. CKC1-MT023 validation fixtures for source hash and timing anchor preservation.
24. CKC1-MT024 migration note from Stage/media stubs into CKC runway.

### WP-CKC-002-Atelier-Lens-Studio-Runtime-v1

25. CKC2-MT001 source map verification for Atelier/Lens/Photo/Studio stubs.
26. CKC2-MT002 Atelier-Lens role claiming requirement reconstruction.
27. CKC2-MT003 SceneState preservation contract.
28. CKC2-MT004 ConflictSet preservation contract.
29. CKC2-MT005 selection-scoped patch law reuse from Atelier Collaboration Panel.
30. CKC2-MT006 role suggestion provenance schema reuse.
31. CKC2-MT007 LensExtractionTier schema adoption.
32. CKC2-MT008 Tier1 default enforcement tests.
33. CKC2-MT009 Tier0/Tier2 explicit override trace tests.
34. CKC2-MT010 ViewMode projection filter reuse.
35. CKC2-MT011 SFW hard-drop projection conformance.
36. CKC2-MT012 raw artifact immutability under ViewMode switching.
37. CKC2-MT013 Photo Studio skeleton surface contract.
38. CKC2-MT014 Photo Studio thumbnail integration decision.
39. CKC2-MT015 Photo Studio recipe model preservation.
40. CKC2-MT016 Studio runtime identity contract.
41. CKC2-MT017 Canvas/Excalidraw runtime mapping.
42. CKC2-MT018 Lens/Atelier runtime mapping.
43. CKC2-MT019 Studio job kind and workflow node inventory.
44. CKC2-MT020 DCC/operator projection requirements.
45. CKC2-MT021 Flight Recorder event linkage requirements.
46. CKC2-MT022 Locus/task-board/WP correlation requirements.
47. CKC2-MT023 PostgreSQL-only state contract with no-SQLite tripwires.
48. CKC2-MT024 GUI visual evidence requirement import from Kernel visual stubs.

### WP-CKC-003-Loom-Archive-Media-Intelligence-v1

49. CKC3-MT001 source map verification for Loom/media/video stubs.
50. CKC3-MT002 active Media Downloader v2 dependency check.
51. CKC3-MT003 Media Downloader v1 source-mode preservation matrix.
52. CKC3-MT004 output root and output layout preservation.
53. CKC3-MT005 captions VTT sidecar preservation.
54. CKC3-MT006 forum/blog full-resolution image preservation.
55. CKC3-MT007 downloader queue/progress evidence contract.
56. CKC3-MT008 LoomBlock wrapper contract for video assets.
57. CKC3-MT009 Asset versus artifact identity reconciliation.
58. CKC3-MT010 content hash dedup across repeated promotions.
59. CKC3-MT011 captions metadata sidecar linkage.
60. CKC3-MT012 info.json provenance linkage.
61. CKC3-MT013 poster frame extraction policy.
62. CKC3-MT014 ffmpeg/ffprobe capability gate tests.
63. CKC3-MT015 preview status and thumbnail asset linkage.
64. CKC3-MT016 ASR fallback job linkage.
65. CKC3-MT017 caption fast-path transcript derivation.
66. CKC3-MT018 transcript chunking and indexing policy.
67. CKC3-MT019 tag and mention edge preservation.
68. CKC3-MT020 AI-suggested tag confirmation gate.
69. CKC3-MT021 Lens transcript query facet integration.
70. CKC3-MT022 large-library batch resume and no-UI-lock validation.
71. CKC3-MT023 cloud transcription opt-in capability gate.
72. CKC3-MT024 visual validation evidence for archive UI and previews.

## Validation Needed

- Validate JSON map schema with a JSON parser.
- Confirm all listed source paths still exist before promotion.
- Reconcile task board, registry, and build order state before creating CKC packets.
- For validated/superseded stubs, verify active packet paths before using them as implementation authority.
- For CKC GUI work, attach screenshot/visual evidence through the inherited Kernel visual validation path.
