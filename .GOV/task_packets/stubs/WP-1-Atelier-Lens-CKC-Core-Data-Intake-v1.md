# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- This stub is planning-only. It authorizes no product code changes.

---

# Work Packet Stub: WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1

## STUB_METADATA
- WP_ID: WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1
- BASE_WP_ID: WP-1-Atelier-Lens-CKC-Core-Data-Intake
- CREATED_AT: 2026-05-16T05:05:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- STUB_FORMAT_VERSION: 2026-04-06
- BUILD_ORDER_DOMAIN: ATELIER_LENS
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: CRITICAL
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Atelier-Lens-Consolidation-v1, WP-1-Artifact-System-Foundations-v1, WP-KERNEL-001-Event-Ledger-Session-Broker-v1, WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
- BUILD_ORDER_BLOCKS: WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1, WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- PRODUCT_REFERENCE: .GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md
- SOURCE_GREENROOM_ROOT: .GOV/reference/ckc_atelier_lens_consolidation
- SOURCE_CKC_CODE: D:/Projects/LLM projects/CastKit-Codex/CKC_main
- SOURCE_CKC_SPEC: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/spec/CastKit_Codex_Spec_v00.075.md
- SOURCE_CKC_TASKBOARD: D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/taskboard/TASK_BOARD.md

## INTENT (DRAFT)
- What: Build the Handshake-native core Atelier/Lens + CKC data/intake foundation as a planning packet. This folds CKC character sheets, media/DAM, intake/inbox, collections/contact sheets, sidecars/versioning/recovery, search/tag/similarity, exports/backups/share-packs, and no-rewrite text preservation into existing Atelier/Lens, Photo Studio, Loom, Stage, ArtifactStore, LensExtractionTier, and LensViewMode intent.
- Why: CKC evolved the same prompt-diary/Atelier/Lens goal outside Handshake. Its data model and operator workflows must be preserved, but implementation must be translated into Handshake primitives before any runtime work starts.
- No-code stance: This stub creates only future work scope and microtasks. It does not implement Rust, Tauri, React, Python, migrations, tests, GUI, or product behavior.
- Model/execution separation: The future implementation must keep LLMs as proposers/planners/operators through governed AI Jobs, Workflow Engine nodes, Locus/MT records, tool policies, receipts, and validation. Product execution happens through Handshake runtime surfaces, not direct LLM file writes or CKC-style process-local authority.

## SOURCE_COVERAGE_STATUS (DRAFT)
- Coverage audit: `.GOV/reference/ckc_atelier_lens_consolidation/wp-stub-coverage-audit-20260516.md`.
- This stub owns the core data/intake/media/search/export layer for the three-stub CKC fold-in family.
- This stub must be self-contained at activation time. A future no-context model may use the greenroom files for evidence, but the source intent below is not optional or merely linked.
- Old stubs remain in place. This stub does not delete, archive, supersede, or activate them. It preserves their intent for a future signed packet.
- Repair status: this stub was audited before microtask generation and needed more detail. The repair payload below is normative planning input for activation: no MT file generation may start until each repair row is either folded into this stub, assigned to another one of the three CKC stubs, or explicitly marked operator-decision-needed.

## FOLDED_SOURCE_STUBS_FULL_PAYLOAD (DRAFT)
- `WP-1-Atelier-Lens-v2`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Atelier-Lens-v2.md`, `.GOV/task_packets/stubs/WP-1-Atelier-Lens-v2.contract.json`.
  - Lifecycle: stub backlog; base `WP-1-Atelier-Lens` is superseded.
  - Preserved original intent: additive remediation for failed or gapped Atelier/Lens work.
  - Preserved scope: role claiming, SceneState, ConflictSet, role/provenance behavior, and Lens proposal workflows.
  - Preserved acceptance need: activation must explicitly reconstruct missing behavior because the contract is sparse. A no-context activation model must inspect the prior packet `.GOV/task_packets/WP-1-Atelier-Lens.md` if present and extract the intended role claiming flow, SceneState shape, ConflictSet shape, provenance rules, failure cases, and validation evidence; if the prior packet is absent or ambiguous, the activation refinement must include an unresolved-source row rather than inventing behavior.
  - Preserved risk: sparse machine contract can cause intent loss if only the JSON is read.
- `WP-1-Photo-Studio-v2`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Photo-Studio-v2.md`, `.GOV/task_packets/stubs/WP-1-Photo-Studio-v2.contract.json`.
  - Lifecycle: stub backlog; base `WP-1-Photo-Studio` is superseded.
  - Preserved original intent: additive remediation for failed or gapped Photo Studio work.
  - Preserved scope: skeleton surface, thumbnails, recipes, media viewer/DAM responsibility, and future recipe persistence/use.
  - Preserved acceptance need: skeleton visibility, thumbnail generation or display, and recipe persistence/use must be re-derived in the CKC refinement. A no-context activation model must inspect the prior packet `.GOV/task_packets/WP-1-Photo-Studio.md` if present and extract the intended skeleton surface, thumbnail/proxy lifecycle, recipe record format, recipe apply/replay behavior, and validation evidence; if the prior packet is absent or ambiguous, the activation refinement must carry an unresolved-source row.
  - Preserved risk: overlap with Loom preview thumbnails and media poster frames requires a shared preview boundary.
- `WP-1-Atelier-Collaboration-Panel-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Atelier-Collaboration-Panel-v1.md`, `.GOV/task_packets/stubs/WP-1-Atelier-Collaboration-Panel-v1.contract.json`.
  - Lifecycle: validated/done as active packet; retained here as source evidence and safety law.
  - Preserved scope: selection-scoped role suggestions, multi-suggestion review, strict range-bounded patching for Monaco/Docs, provenance logging, validator rejection of out-of-selection patches.
  - Preserved acceptance: applying suggestions must never change text outside the selected span; validators reject out-of-range patches; applied patches have provenance and visible evidence refs.
  - Preserved risk: editor range semantics and boundary normalization can create off-by-one loopholes.
  - Handling: baseline dependency, not reopened wholesale.
- `WP-1-Lens-Extraction-Tier-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Lens-Extraction-Tier-v1.md`, `.GOV/task_packets/stubs/WP-1-Lens-Extraction-Tier-v1.contract.json`.
  - Lifecycle: stub backlog.
  - Preserved scope: Tier0/Tier1/Tier2 model, Tier1 default, explicit Tier0/Tier2 override, requested-versus-effective trace, invalid-tier validation.
  - Preserved acceptance: Tier1 defaults when unspecified; Tier0/Tier2 override intent is trace-visible; validators reject misuse.
  - Preserved risk: confusion with `content_tier`; implicit Tier2 creep through heuristics.
  - Handling: folded as search/extraction control, orthogonal to safety/view projection.
- `WP-1-Lens-ViewMode-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Lens-ViewMode-v1.md`, `.GOV/task_packets/stubs/WP-1-Lens-ViewMode-v1.contract.json`.
  - Lifecycle: validated/done as active packet; retained as source evidence and projection law.
  - Preserved scope: NSFW default, explicit SFW toggle, hard-drop SFW projection, immutable raw and derived artifacts, trace-visible ViewMode metadata filter.
  - Preserved acceptance: SFW ViewMode never shows non-sfw items; toggling ViewMode does not mutate stored artifacts; QueryPlan or RetrievalTrace records ViewMode as a filter.
  - Preserved risk: inconsistent enforcement across surfaces; accidental mutation of stored artifacts.
  - Handling: baseline dependency; CKC media review consumes the projection rule.
- `WP-1-Stage-Media-Artifact-Portability-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Stage-Media-Artifact-Portability-v1.md`, `.GOV/task_packets/stubs/WP-1-Stage-Media-Artifact-Portability-v1.contract.json`.
  - Lifecycle: stub backlog; Build Order rank 1; value high; risk high; blocked.
  - Preserved scope: shared artifact portability contract across Stage session records, media capture sessions, debug bundle export, storage portability rules, session/auth/materialization bounded export anchors.
  - Preserved acceptance: Stage and Media Downloader link through portable artifact and retention contracts; later packets reuse one bounded manifest contract; storage swaps do not redefine evidence semantics.
  - Preserved dependencies: `WP-1-Handshake-Stage-MVP`, `WP-1-Media-Downloader`, `WP-1-Artifact-System-Foundations`, `WP-1-Storage-Trait-Purity`.
  - Preserved unlocks: `WP-1-Media-Downloader-Loom-Bridge`, `WP-1-Video-Archive-Loom-Integration`.
  - Preserved risk: Stage and media artifacts could fork incompatible manifests; capture-session provenance could be omitted.
- `WP-1-Stage-ASR-Transcript-Lineage-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Stage-ASR-Transcript-Lineage-v1.md`, `.GOV/task_packets/stubs/WP-1-Stage-ASR-Transcript-Lineage-v1.contract.json`.
  - Lifecycle: stub backlog; Build Order rank 9; value high; risk high; blocked.
  - Preserved scope: source media artifact -> ASR input -> transcript artifact -> searchable consumer; stable source hash, media probe facts, capture/session provenance, timing anchors, recorder-visible events, storage-portable transcript linkage.
  - Preserved acceptance: Stage media and ASR transcripts use a single backend lineage contract; later packets reuse source hash, timing anchor, and provenance contracts across Stage, ASR, Loom, archive flows.
  - Preserved dependencies: `WP-1-Handshake-Stage-MVP`, `WP-1-ASR-Transcribe-Media`, `WP-1-Media-Downloader`, `WP-1-Artifact-System-Foundations`.
  - Preserved risk: transcript text can survive while timing anchors or provenance are lost.
- `WP-1-ASR-Transcribe-Media-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-ASR-Transcribe-Media-v1.md`, `.GOV/task_packets/stubs/WP-1-ASR-Transcribe-Media-v1.contract.json`.
  - Lifecycle: stub backlog; Build Order rank 36; value medium; risk medium.
  - Preserved scope: local-first `asr_transcribe` Workflow Engine job, deterministic audio extraction through ffmpeg, transcript artifacts in JSON and readable text, timing metadata, attachment to LoomBlocks or full-text index, capability gating, no cloud transcription by default, Flight Recorder progress/events.
  - Preserved acceptance: videos without captions produce timestamped transcript artifacts searchable through Loom; captioned videos can skip ASR and ingest captions; jobs are cancellable, resumable where feasible, capability-gated, and leak-safe.
  - Preserved technical decisions still required: activation must decide and name the local ASR engine (`whisper.cpp` or equivalent), ffmpeg/ffprobe provisioning/pinning/discovery, canonical transcript format or multi-format bundle (JSON segments, readable text, WebVTT ingest path), chunking/resource bounds for long media, and index-retention policy.
  - Preserved risk: long-video performance/memory and transcript indexing cost.
- `WP-1-Video-Archive-Loom-Integration-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Video-Archive-Loom-Integration-v1.md`, `.GOV/task_packets/stubs/WP-1-Video-Archive-Loom-Integration-v1.contract.json`.
  - Lifecycle: stub backlog; Build Order rank 10; value high; risk high; blocked.
  - Preserved scope: archived/imported videos become Loom library objects with searchable transcripts, captions sidecars, tags/mentions, thumbnails/proxies, transcript/caption Lens retrieval, Flight Recorder events.
  - Preserved acceptance: importing video folders creates stable LoomBlocks with `title`, `imported_at`, source metadata, thumbnails/proxies, and transcript/caption references; transcript/caption Documents attach as referenced entities rather than duplicated text blobs; transcripts are searchable and time-offset linked; tags/mentions use LoomEdges; large libraries stay usable through batch progress/resume and never lock the UI.
  - Preserved dependencies: `WP-1-Media-Downloader`, `WP-1-Loom-MVP`, `WP-1-Media-Downloader-Loom-Bridge`, `WP-1-Loom-Preview-VideoPosterFrames`, `WP-1-ASR-Transcribe-Media`.
  - Preserved risk: transcript indexing cost, captions versus ASR duplicates, privacy/cloud-send confusion.
- `WP-1-Loom-MVP-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Loom-MVP-v1.md`, `.GOV/task_packets/stubs/WP-1-Loom-MVP-v1.contract.json`.
  - Lifecycle: validated/done as active packet; retained as baseline source evidence.
  - Preserved scope: LoomBlocks, LoomEdges, stable UUID identity, mentions, tags, All/Unlinked/Sorted/Pins views, backlinks panel, file import, SHA-256 dedup, Tier-1 previews, basic search, FR-EVT-LOOM events.
  - Preserved acceptance: imports create LoomBlocks; dedup prevents duplicates; mentions/tags create stable LoomEdges; backlinks update; Tier-1 search works; FR-EVT-LOOM events surface in operator history.
  - Preserved risk: anchor drift, dedup identity errors, preview generation degrading UI responsiveness.
  - Handling: baseline dependency; CKC extends without replacing Loom identity semantics.
- `WP-1-Loom-Storage-Portability-v4`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Loom-Storage-Portability-v4.md`, `.GOV/task_packets/stubs/WP-1-Loom-Storage-Portability-v4.contract.json`.
  - Lifecycle: task board says validated; registry text is mixed and needs later cleanup.
  - Preserved scope: PostgreSQL-only evidence for graph traversal, directional edges, metrics recomputation, and source-anchor durability; older dual-backend intent is superseded.
  - Preserved acceptance: remediation ties to current clauses and evidence; concrete defects are proven by PostgreSQL-only checks; if no defect remains, refinement narrows scope.
  - Preserved risk: old v3 mixed concrete storage work with broad closure claims; registry inconsistency can confuse agents.
  - Handling: baseline dependency after state clarification; never revive SQLite-era dual-backend patterns.
- `WP-1-Loom-Preview-VideoPosterFrames-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Loom-Preview-VideoPosterFrames-v1.md`, `.GOV/task_packets/stubs/WP-1-Loom-Preview-VideoPosterFrames-v1.contract.json`.
  - Lifecycle: stub backlog; Build Order rank 20; value high; risk medium.
  - Preserved scope: deterministic representative frame extraction, resize to ThumbnailSpec, preferred encoding, derived preview asset storage, LoomBlock link, ffmpeg/ffprobe capability gate, Flight Recorder events.
  - Preserved acceptance: video imports/promotions produce retrievable thumbnails and generated preview status; generation is non-blocking, resumable, idempotent; invalid files fail with clear error codes.
  - Preserved risk: ffmpeg-version determinism; large-video performance/cancellation.
- `WP-1-Media-Downloader-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Media-Downloader-v1.md`, `.GOV/task_packets/stubs/WP-1-Media-Downloader-v1.contract.json`.
  - Lifecycle: task board says v1 is superseded and v2 is validated/done; registry active path is `.GOV/task_packets/WP-1-Media-Downloader-v2.md`.
  - Preserved historical source modes: YouTube, Instagram, forum/blog topic images, generic video URLs, unified queue/progress, user-configurable output root, public/account modes, captions sidecars, forum full-resolution crawling, resumable queue, rate limiting, deterministic naming and sidecars, optional Director transcode, Bronze/Raw materialization, Flight Recorder events, capability gating.
  - Preserved output layout: `Handshake_Output/media_downloader/{youtube|instagram|forumcrawler|videodownloader}/...` as the historical materialization convention; Handshake activation must translate this to `OutputRootDir`/ArtifactStore semantics without losing source-kind subfolders, deterministic names, sidecar manifests, or resume markers.
  - Preserved auth/secrets rules: no-account mode and account/cookie mode exist; account/cookie mode uses governed session/cookie capture and encrypted secret storage; Handshake must not collect forum or platform passwords in a Handshake-owned form; cookie jars are never written to ordinary output folders or logged.
  - Preserved forum crawler rules: crawl paginated topics, prefer full-resolution originals behind thumbnails through anchor-follow/srcset/data-fullsize/heuristic rewrites, skip profile avatars, author profile pictures, emojis, UI chrome, and thumbnails when a full-resolution target exists; support bounded max pages, polite rate limits, site adapter or selector config, and explicit allowlist posture.
  - Preserved acceptance: archive resumes and avoids duplicates; captions saved as VTT; transcode deterministic; network/exec actions are capability-gated; UI shows per-item and batch progress; output sidecars include source URL, retrieval time, source kind, stable IDs where available, hash, byte size, caption tracks, and error/skipped reasons.
  - Preserved risk: platform policy changes, fragile parsing, large storage, partial downloads, caption variability, forum variance, selector-config drift, accidental credential leakage, and SSRF/local-network probing.
  - Handling: historical source only; use active v2 as dependency if implementation begins.
- `WP-1-Media-Downloader-v2`
  - Source paths: `.GOV/task_packets/WP-1-Media-Downloader-v2/packet.md`, `.GOV/task_packets/WP-1-Media-Downloader-v2/packet.json`, `.GOV/refinements/WP-1-Media-Downloader-v2.md`.
  - Lifecycle: validated/done active packet for base `WP-1-Media-Downloader`; this stub must use v2, not v1, if downloader behavior is activated.
  - Preserved active contract: unified Media Downloader worksurface; YouTube/Instagram/forum crawler/generic video in one delivery; `OutputRootDir` user-configurable default materialization root; per-kind output directories; stable URL normalization/dedup/queue counts; pause/resume/cancel/retry; concurrency default 4 with range 1..16; forum default `max_pages=1500` and hard cap 5000; `.part` download finalization; ffprobe validation for generic video; Stage Sessions-based auth; multiple persistent sessions selectable per job; no password collection; cookie jars as high-classified non-exportable artifacts; sanitized Flight Recorder events; capability profiles for `net.http`, `proc.exec:yt-dlp`, `proc.exec:ffmpeg`, `proc.exec:ffprobe`, `secrets.use`, and `fs.write:artifacts`.
  - Preserved schemas to carry into activation: `hsk.media_downloader.batch@v0`, `hsk.media_downloader.control@v0`, `hsk.media_downloader.result@v0`, `media_sidecar.json`, `forumcrawler_manifest.json|csv`, sanitized `media_downloader.job_state`, `media_downloader.progress`, and `media_downloader.item_result` events.
  - Preserved risk controls: deny localhost/private IP targets by default, enforce allowlists, pin/discover external tools, avoid shell invocation, validate media containers, enforce disk/CPU caps, and never materialize secrets under `OutputRootDir`.
- `WP-1-Media-Downloader-Loom-Bridge-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Media-Downloader-Loom-Bridge-v1.md`, `.GOV/task_packets/stubs/WP-1-Media-Downloader-Loom-Bridge-v1.contract.json`.
  - Lifecycle: stub backlog; Build Order rank 30; value high; risk medium.
  - Preserved scope: bridge completed downloader artifacts/materialized paths into workspace Asset records and LoomBlock wrappers; dedup by content hash; preserve captions and metadata sidecars; enqueue previews; ingest captions into searchable layer; emit Flight Recorder promotion/dedup/linking/captions/preview events.
  - Preserved acceptance: completed YouTube archive promotes into a dedup-stable LoomBlock with media Asset, captions sidecars, searchable caption text, queued/generated preview; operations are capability-gated and leak-safe.
  - Preserved risk: artifact versus asset store duplication, large-file non-streaming imports, caption language/track variance.
- `WP-1-Calendar-Lens-v3`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Calendar-Lens-v3.md`, `.GOV/task_packets/stubs/WP-1-Calendar-Lens-v3.contract.json`.
  - Handling: separate Lens pattern source, not CKC media scope.
  - Preserved pattern: day/week/month or staged view, source/date filters, title search, backend source/event query, stable IDs, provenance, optional read-only overlay into timeline/Flight Recorder.
  - Preserved acceptance: user can open Calendar Lens, see events for a selected window, add a source, filter by source, and reload without drift.
  - Preserved risk: timezone/DST and accidental indexing of private events.
- `WP-1-Artifact-System-Foundations-v1`
  - Source paths: `.GOV/task_packets/stubs/WP-1-Artifact-System-Foundations-v1.md`, `.GOV/task_packets/stubs/WP-1-Artifact-System-Foundations-v1.contract.json`.
  - Handling: foundational dependency.
  - Preserved scope: artifact store bootstrap, manifests, SHA-256, single atomic Materialize API, retention/pinning/GC, no random filesystem side effects.
  - Preserved acceptance: all export paths use one atomic materialize implementation; artifacts and bundles have stable manifests and hashes; retention/GC never deletes pinned items and emits visible report.
  - Preserved risk: competing materialize implementations and retention/GC data-loss bugs.

## CKC_GREENROOM_PAYLOAD_OWNED (DRAFT)
- `EVOL-001` stable public character IDs separate from internal IDs.
  - Source evidence: `app/backend/db.js` `Character.public_id`, `app/backend/library.js` public-id helpers, `backend_public_character_id.test.js`.
  - Requirement: preserve stable operator-facing character labels without exposing storage keys; public IDs are system-managed and not ordinary sheet text.
- `EVOL-002` typed character sheet parser with union and block-list fields.
  - Source evidence: `templateParser.js`, `sheet.js`, `validation.js`, `BlockListEditor.tsx`, `template_parser_unions.test.js`, `validation_field_types.test.js`, `block_list_validation.test.js`.
  - Requirement: preserve typed fields, descriptor fallbacks, score normalization, nested block editing, adult/explicit production fields, and byte/text preservation; port semantics, not JS implementation.
- `EVOL-003` append-only sheet versions with selective apply/revert.
  - Source evidence: `SheetVersion` table, `SheetVersionTools.tsx`, `SheetIngestMergeTools.tsx`, `saveCharacter` flows.
  - Requirement: no model/import edit overwrites prior sheet values silently; revert creates a new version with provenance.
- `EVOL-004` bulk character operations.
  - Source evidence: `LibraryView.tsx`, `BulkFieldEditDialog.tsx`, `BulkTagDialog.tsx`, `BatchExportDialog.tsx`, CKC taskboard `WP-0090`.
  - Requirement: multi-select bulk tags/fields, batch exports, trash/restore, EventLedger receipts, and validation before mass mutation.
- `EVOL-004A` CKC library carousel/frontpage/rating/favorite review signals.
  - Source evidence: `src/ui/views/LibraryView.tsx` carousel/inbox/collections lanes, `src/ui/components/MediaPane.tsx` favorite/rating/frontpage/carousel controls, `app/backend/library.js` favorite/rating/source/search queries.
  - Requirement: preserve media review affordances as data/projection requirements: favorites, 0..5 ratings, `frontpage` and `carousel` tags, global carousel ranking that prefers frontpage/favorite/rating, batch metadata edits, and keyboard rating behavior. These are not decorative UI details; they drive selection, export, and prompt-diary review.
- `EVOL-005` persistent intake batches.
  - Source evidence: `IntakeBatch`, `IntakeBatchItem`, `createIntakeBatch`, `classifyIntakeBatch`, CKC taskboard `WP-0124`, `WP-0125`, `WP-0126`.
  - Requirement: pending/inbox lifecycle, source preservation, loose/linked profile modes, idempotent accept/reject/defer, and resume after route switch or restart.
- `EVOL-006` contact sheets as artifacts.
  - Source evidence: `ContactSheet` table, `createContactSheet`, CKC taskboard `WP-0129`, `WP-0136`.
  - Requirement: contact sheets are reproducible artifacts with source IDs, hashes, layout metadata, collection links, and deferred raster export research.
- `EVOL-007` OpenPose sidecars hidden from normal galleries.
  - Source evidence: `ImageAsset.media_role`, `source_image_id`, `openpose_png_path`, `listOpenposeSidecars`, CKC taskboard `WP-0130`, `WP-0132`.
  - Requirement: Core owns projection law: sidecars remain artifacts, searchable by relation, hidden from normal galleries unless intentionally projected; Pose/ComfyUI owns production/schema.
- `EVOL-015` identity-decoupled media filenames.
  - Source evidence: `backend_identity_decoupling.test.js`, `ingestImageSourcingTask`, content-hash naming.
  - Requirement: filenames/events must not leak character names or sensitive sheet fields; use no-space content-addressed artifact names.
- `EVOL-016` global search with snippets and jump targets.
  - Source evidence: `globalSearch`, `GlobalSearchModal.tsx`, `SavedSearch`, CKC taskboard `WP-0083`.
  - Requirement: search across sheets, notes, images, moodboards with snippets and jump targets; use Handshake search/index architecture, never SQLite FTS.
- `EVOL-017` tag manager, saved searches, palettes, dHash similarity.
  - Source evidence: `TagRule`, `SavedSearch`, `palette.js`, `dhash.js`, CKC taskboard `WP-0059`, `WP-0066`, `WP-0067`, `WP-0089`.
  - Requirement: tags, saved searches, palettes, dHash similarity, AI tag suggestions, color filters, similar-image grouping, tag rules, and bulk tag application are derived projections with rebuild receipts and visible validation. AI suggestions remain proposed/derived until explicitly applied.
- `EVOL-018` moodboard canvas inside character workflow.
  - Source evidence: `MoodboardDoc`, `MoodboardCanvas.tsx`, CKC taskboard `WP-0071` through `WP-0082`.
  - Requirement: preserve structured JSON, layers/folders, canvas images, shapes, connectors, text items, tags/search, undo/redo/history, locked/hidden layers, guides, alignment/arrange behavior, PNG/PDF export hooks, backlinks/jump targets, and prompt-diary/visual-planning intent inside Atelier/Lens. Do not collapse moodboards into a generic attachment list.
- `EVOL-018A` docs, stories, corkboard/outliner, and relationship graph.
  - Source evidence: `CharacterView.tsx` docs drawer, notes/stories/moodboard tabs, story cards, backlinks/outbound links, `StoryBeat`/`CharacterScript` APIs, `CharacterRelation` table/preload/main handlers, automation manual `relationships-collections-reference`.
  - Requirement: preserve character-scoped notes, stories, story cards, story beats, per-character scripts, bracket-link backlinks, outbound links, docs drawer filters/tags, relationship rows with type/notes/target character, and future relationship-map projection. These belong to Core because they are knowledge/data graph primitives used by Atelier/Lens; Diagnostics only owns model command visibility over them.
- `EVOL-018B` URL/clipboard/inbox import provenance.
  - Source evidence: `LibraryView.tsx` `importClipboardImage`, `importFromUrl`, `scanInbox`, `app/backend/library.js` `source_url`, `source_path`, `source_note`, `review_status`, pending/inbox rows, `IngestionRejection`.
  - Requirement: preserve clipboard, URL, folder scan, inbox, pending/rejected/accepted, source URL/path/note, duplicate skip, source contact sheet/task/run refs, and rejection audit behavior. Activation must define how untrusted URLs are capability-gated, how provenance survives moves, and how pending items can be accepted/rejected/deferred without source loss.
- `EVOL-022` filesystem health and recoverable deletion.
  - Source evidence: `filesystemHealthCheck`, `deletionImpactPreview`, `archiveImages`, CKC taskboard `WP-0127`, `WP-0135`.
  - Requirement: missing-file diagnostics, deletion impact preview, archive/restore decisions, and EventLedger records for every destructive or recovery action.
- `EVOL-023` backup version traceability and orphan adoption.
  - Source evidence: `backup.js`, `resetModes.js`, `adoptOrphanImages`, CKC taskboard `WP-0105`, `WP-0106`.
  - Requirement: artifact manifests/checksums, backup version guards, orphan adoption receipts, and no CKC folder authority.
- `EVOL-024` web portfolio and share pack exports.
  - Source evidence: `app/templates/web-portfolio`, `exportWebPortfolio`, `exportSharePack`, CKC taskboard `WP-0087`, `WP-0063`.
  - Requirement: portable handoff outputs through ArtifactStore/Workflow Engine, safe subsets, LLM packs, manifests, checksums, and no product outputs under `.GOV`.
- `EVOL-024A` build, package, install, release, and portable data-root obligations.
  - Source evidence: CKC taskboard `WP-0001`, `WP-0008`, `WP-0017`, `WP-0019`, `WP-0020`, `WP-0044`, `WP-0086`, `WP-0105`, `WP-0120`, `WP-0121`; `CKC_main/scripts/package_win.ps1`, `scripts/package_mac.sh`, `scripts/release.ps1`, `scripts/installer_custom.nsh`; CKC README stale spec pointer.
  - Requirement: when CKC behavior becomes Handshake implementation work, activation must preserve packaging/release lessons as acceptance gates: build outputs outside the product repo, clean-repo packaging guard, relative asset paths for packaged windows, no drive-letter assumptions, portable/dev versus SemVer release artifact folders, installer/update/reinstall/light-reset/full-reset modes, orphan manifest/adoption flow, and spec/manual/readme drift checks. Core owns data-root/reset/orphan behavior; Diagnostics owns build/release evidence and manual visibility.
- `EVOL-025` hybrid CRDT/event-log policy.
  - Source evidence: `EntityRevision`, `ProductEvent`, CKC taskboard `WP-0134`, automation manual.
  - Requirement: Core records PostgreSQL/EventLedger authority and optimistic revision behavior; Diagnostics owns model/session/parallel operation projection; CRDT only for safe merge shapes.

## GREENROOM_OVERLAP_ROWS_OWNED (DRAFT)
- `OVR-001` character sheets and Atelier identity: fold stable IDs, protected fields, append-only sheet versions, selective merge/apply, byte-preserved user text, role/provenance.
- `OVR-002` media viewer / DAM / Photo Studio: fold browsing, thumbnails, metadata, provenance, missing-file diagnostics, archive/restore, sidecar hiding.
- `OVR-003` intake / inbox / pending review: fold persistent batches, accept/reject/pending, loose/linked modes, source preservation, character/sheet/collection linkage, resume after route switch/restart.
- `OVR-004` collections and contact sheets: fold notes/tags, optional character/sheet-version links, contact-sheet manifests, source IDs/hashes, layout metadata, deferred raster export.
- `OVR-005` sidecars, versioning, recovery: Core owns visibility projection, archive/restore, revision checks, append-only events, deletion preview, and no silent source deletion; Pose/ComfyUI owns pose sidecar production/schema.
- `OVR-008` search, tags, links, similarity: fold snippets, jump targets, tag manager, saved searches, backlinks, palettes, dHash similarity, AI tag suggestions; no SQLite FTS.
- `OVR-009` docs, stories, moodboards, prompt diary intent: fold docs inside character workflow, moodboard structured JSON, layers/folders, corkboard/outliner, links/backlinks, exports, text preservation.
- `OVR-011` exports, backups, share packs, web portfolio: fold no-space names, safe subsets, LLM packs, manifests, backup guards, orphan adoption, checksums, offline portfolio intent; outputs outside `.GOV`.
- `OVR-012` parallel editing / event log / revisions: Core owns data authority and optimistic revisions; Diagnostics owns sessions/leases/model operation; PostgreSQL/EventLedger remain authority.

## CROSS_STUB_BOUNDARIES (DRAFT)
- Pose/OpenPose sidecar production, identity profile payloads, ComfyUI receipts, workflow replay, and PoseKit calibration debt belong to `WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1`.
- Model manual, action catalog, leases, command logs, visual capture, diagnostic bundles, DCC/Locus/Flight Recorder projections, and non-focus automation belong to `WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1`.
- Core owns artifact/data/source-of-truth contracts consumed by both downstream stubs.
- No fourth CKC fold-in stub may be created unless the operator explicitly approves breaking the three-stub limit.

## HANDSHAKE_TRANSLATION (DRAFT)
- Module boundary candidates: `atelier_core`, `atelier_sheet`, `atelier_media`, `atelier_intake`, `atelier_collections`, `atelier_sidecars`, `atelier_search`, `atelier_exports`, and `kernel_event_bridge`.
- Desktop shell: Tauri command facade only; CKC Electron main/preload IPC is evidence only.
- Backend coordinator: Rust domain services and APIs own durable domain behavior. CKC JavaScript backend modules are source evidence, not code to copy.
- AI orchestration: Python/AutoGen/LangGraph and AI Job/Workflow Engine may propose/extract/transform through capability gates and receipts, but they do not hold product authority.
- Frontend: React/TypeScript projections may later display core data, but UI is not this stub's implementation center.
- Storage: PostgreSQL is primary runtime authority. SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Collaboration: Yjs/CRDT only for collaboration-safe document/workspace shapes; authoritative records remain PostgreSQL-backed with EventLedger facts.
- Evidence: Flight Recorder, EventLedger, ArtifactStore manifests, Locus/MT links, and DCC projections must make every model or tool action observable, attributable, and recoverable.
- Search: preserve CKC search behavior, snippets, jump targets, tags, saved searches, palettes, and similarity, but translate away from SQLite FTS5 to Handshake search/index architecture.
- Paths: D-drive CKC paths are historical evidence only; runtime paths must be repo-relative, artifact-root-relative, or operator-configured.
- Product artifacts: no product outputs under `.GOV`; exports/share packs/backups are ArtifactStore/Workflow Engine jobs with manifests.
- Runtime namespace: no CKC/CastKit product namespace in Handshake runtime; source names may appear only as evidence references.
- Evidence maturity: CKC taskboard `DONE` rows may seed parity fixtures; `REVIEW` rows require review-batch evidence to be checked before being treated as final; `BLOCKED` rows are preserved as unresolved scope; `PLANNED` rows are future/deferred requirements, not done behavior. Activation must not flatten these statuses into one backlog bucket.
- Spec drift: CKC `README.md` currently points at spec v00.063 while the provided current spec is v00.075. Activation must treat README/spec mismatch as source-drift evidence and resolve the equivalent Handshake manual/spec pointer before implementation.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Character identity and sheet domain contract.
  - Template/parser/protected-field requirements.
  - Append-only sheet versioning, selective merge/apply/revert, and no silent rewrite.
  - Media asset/DAM contract: images, thumbnails, metadata, provenance, missing-file diagnostics, sidecars, tags, ratings, favorites, similarity projections.
  - Intake/inbox contract: persistent batches, loose/linked modes, pending review, accept/reject/defer, idempotency, restart/resume.
  - Collections/contact-sheet contract: collection notes/tags/links, contact-sheet artifact manifests, source IDs/hashes, deferred raster export hook.
  - Search/tag/link/similarity contract through Handshake retrieval primitives.
  - Export/backup/share-pack contract through ArtifactStore and Workflow Engine.
  - Media downloader and Loom/archive carry-forward requirements: `OutputRootDir`, Stage Sessions/cookie auth, captions, ASR, title/imported_at/source metadata, transcript/caption reference semantics, no-UI-lock batch import, and downloader schemas/events.
  - CKC docs/stories/moodboard/relationship data graph: notes, stories, story beats, scripts, backlinks/outbound links, moodboard full toolset, relationship rows, and future relationship-map projection requirements.
  - CKC media review details: favorites, ratings, carousel/frontpage tags, palette/color filters, dHash/similar-image projections, AI tag suggestion proposal/apply boundary, source URL/path/note provenance, clipboard/URL/inbox import, pending/rejected/accepted lifecycle.
  - Data-root, backup, installer/reset/orphan-adoption, packaging lesson preservation as future Handshake acceptance gates where this stub owns the data side.
  - Evidence maturity mapping for CKC taskboard statuses: DONE, REVIEW, BLOCKED, PLANNED, and stale-source rows.
  - Product code anchor verification plan for the product worktree.
- OUT_OF_SCOPE:
  - Full GUI rebuild or dockable workspace shell.
  - PoseKit/OpenPose runtime implementation; covered by `WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1`.
  - ComfyUI bridge implementation; covered by `WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1`.
  - Model operation diagnostics; covered by `WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1`.
  - Any CKC direct runtime dependency, CKC namespace authority, Electron IPC, localhost CKC intake authority, or SQLite.

## ACCEPTANCE_CRITERIA (DRAFT)
- A no-context model can identify every source stub and CKC feature folded into this packet.
- Every source stub listed in `FOLDED_SOURCE_STUBS_FULL_PAYLOAD` has preserved original intent, scope, acceptance need, dependencies or lifecycle notes, and risk/handling notes.
- Every owned CKC evolved feature row listed in `CKC_GREENROOM_PAYLOAD_OWNED` is represented as a future requirement, not only a range reference.
- The prior `WP-1-Atelier-Lens` and `WP-1-Photo-Studio` packets are either inspected and their role claiming/SceneState/ConflictSet/skeleton/thumbnails/recipes payload is folded, or the activation refinement carries explicit unresolved-source rows.
- The active `WP-1-Media-Downloader-v2` contract is preserved in enough detail that a no-context model can implement downloader-adjacent integration without reopening the active packet: output layout, Stage Sessions/cookie rules, schemas, progress/concurrency, allowlists/capabilities, sanitized telemetry, forum full-resolution crawl, avatar/emoji/profile-image skips, and secret-handling rules are all visible.
- ASR and Video Archive/Loom preservation is concrete: ffmpeg/ffprobe provisioning, local ASR engine choice, canonical transcript format, `title`, `imported_at`, source metadata, caption/transcript reference semantics, and batch/no-UI-lock constraints are captured.
- CKC full-app coverage gaps are explicitly represented: media review/rating/carousel/frontpage, URL/clipboard/inbox provenance, relationship rows/map projection, docs/stories/story beats/scripts, full moodboard toolset, AI tagging/local LLM proposal boundary, package/release/install/reset/orphan lessons, and README/spec drift.
- CKC taskboard statuses remain meaningful: REVIEW rows need evidence review, BLOCKED rows remain unresolved, PLANNED rows remain deferred, and DONE rows can seed parity fixtures.
- Every owned overlap row listed in `GREENROOM_OVERLAP_ROWS_OWNED` has an owner boundary and required handling.
- All core data/intake requirements cite source evidence from greenroom artifacts, CKC code/spec/taskboard, or existing Handshake stubs.
- Every overlapping CKC/Atelier responsibility has one owner boundary: core data/intake, pose/ComfyUI pipeline, or model workflow diagnostics.
- Future implementation path is Handshake-native: Tauri/Rust/React/Python orchestration/PostgreSQL/Yjs/EventLedger/ArtifactStore/Flight Recorder/Locus/DCC.
- LLM and execution separation is explicit: LLMs create proposals/jobs/tool calls; Handshake executes through governed runtime surfaces and records evidence.
- SQLite is rejected in runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Electron authority, CKC namespace authority, localhost intake authority, `.GOV` product outputs, machine-local runtime paths, and direct LLM execution are explicitly rejected.
- Product code anchor verification is required before activation because the current worktree is governance-kernel-only.

## MICROTASKS (DRAFT)

- Draft MT authority: non-executable planning only; official MT files/contracts are still not generated.
- Replacement rule: the earlier 20-item draft MT list is retired and must not be reused as source for activation.
- DRAFT_MICROTASK_SUITE_PATH: .GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1/MT_SUITE.md
- DRAFT_MICROTASK_COUNT: 80
- OFFICIAL_MICROTASKS_GENERATED: false
- DRAFT_MICROTASK_ACTIVATION_DESTINATION_PATTERN: .GOV/task_packets/<WP_ID>/MT-*.{md,json}
- Fresh no-context MT suite: `.GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1/MT_SUITE.md`.
- Draft MT count: 80.
- Granularity rule: Activation Manager must split any MT further if it touches unrelated files, crosses owner boundaries, or cannot be executed by a no-context local/small cloud model from that MT alone.
- Activation rule: convert these draft MTs into official `.GOV/task_packets/<WP_ID>/MT-*.json` and `.md` only after refinement, USER_SIGNATURE, and official packet creation.

## RISKS / UNKNOWNs (DRAFT)
- Risk: CKC data convenience features get treated as optional extras. Mitigation: every EVOL row is carried into MT acceptance or explicitly assigned to another stub.
- Risk: old Atelier/Lens requirements are overwritten by CKC. Mitigation: source stub preservation is MT-001 and acceptance-critical.
- Risk: sparse old stubs hide important prior-packet behavior. Mitigation: activation must inspect prior packets or carry unresolved-source rows for role claiming, SceneState, ConflictSet, skeleton surface, thumbnails, and recipes.
- Risk: active Media Downloader v2 implementation knowledge is lost because only v1 history is read. Mitigation: activation must use v2 packet/refinement as active contract and preserve `OutputRootDir`, Stage Sessions, cookie jars, schemas, progress/concurrency, allowlists, sanitized telemetry, and external-tool gates.
- Risk: REVIEW/BLOCKED/PLANNED CKC rows are treated as DONE parity. Mitigation: evidence maturity mapping is acceptance-critical; blocked/planned work cannot be represented as implemented.
- Risk: CKC app areas outside the first greenroom EVOL rows are compressed away. Mitigation: docs/stories/moodboard/relations, URL import, AI tagging/local LLM, media review, package/release/reset, and README/spec drift are explicit acceptance rows.
- Risk: SQLite returns through tests or compatibility. Mitigation: MT-017 tripwire and absolute rejection wording.
- Risk: implementation starts in governance-kernel worktree. Mitigation: MT-002 requires product worktree path verification before activation.
- Risk: future UI work expands this packet. Mitigation: UI projections only; full GUI/dockable shell remains out of scope.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm operator accepts this as one of no more than three CKC fold-in WP stubs.
- [ ] Read `.GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md` and cited Master Spec sections.
- [ ] Verify product worktree anchors.
- [ ] Produce Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE.
- [ ] Create official task packet and MT files.
- [ ] Keep SQLite forbidden everywhere.
