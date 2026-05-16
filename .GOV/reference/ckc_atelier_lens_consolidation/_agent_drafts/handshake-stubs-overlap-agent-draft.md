---
file_id: handshake-stubs-overlap-agent-draft
file_kind: ckc_atelier_lens_overlap_agent_draft
updated_at: 2026-05-16
status: draft_reference_only
source_artifacts:
  - ../handshake-stub-preservation-map.md
  - ../handshake-stub-preservation-map.json
  - ../greenroom-requirements-register.md
  - ../greenroom-translation-matrix.md
  - ../consolidated-runway.md
  - ../../../task_packets/stubs/WP-1-Atelier-Lens-Consolidation-v1.md
---

# Handshake Stubs Overlap Agent Draft

This draft preserves Atelier/Lens-adjacent Handshake stub intent and extracts overlap against CKC capability clusters. It is not activation authority and does not create a task packet. Official work still requires refinement, operator signature, task packet creation, task-board transition, and traceability updates.

## Non-Negotiable Gate

SQLite is forbidden in any Handshake context: runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temp harnesses, imports, exports, or product paths. CKC SQLite/FTS5/runtime-DDL evidence is requirement-source evidence only and must be translated to PostgreSQL/EventLedger/search-index architecture before any implementation.

## Source Stub Classification

| Source | Classification | Preserved Intent And Scope | Dependencies / Gaps |
|---|---|---|---|
| `WP-1-Atelier-Lens-Consolidation-v1` | baseline | Preservation-first consolidation gate before CKC rebuild stubs; one no-loss register over Atelier/Lens, Photo Studio, Studio, Stage/media, Loom/archive, artifact, visual-debug, and CKC evolved features. | Blocks CKC rebuild stubs until preservation, greenroom review, CKC research, conflicts, and no-SQLite gates are explicit. |
| `WP-1-Atelier-Lens-v2` | folded | Preserve role claiming, `SceneState`, `ConflictSet`, and failed/gapped Atelier/Lens remediation intent. | Sparse machine contract; acceptance must be re-derived from Markdown source. |
| `WP-1-Photo-Studio-v2` | folded | Preserve skeleton surface, thumbnails, recipes, and failed/gapped Photo Studio remediation intent. | Sparse machine contract; boundary needed against Loom previews and media/DAM thumbnail service. |
| `WP-1-Atelier-Collaboration-Panel-v1` | baseline | Reuse selection-scoped role suggestions, multi-suggestion review, Monaco/Docs range-bounded patching, provenance, and validator rejection of out-of-selection patches. | Validated/done baseline; CKC should reuse patch law, not reopen the packet. |
| `WP-1-Lens-Extraction-Tier-v1` | folded | Preserve Tier0/Tier1/Tier2, Tier1 default, explicit override, requested/effective tier trace, and invalid-tier validation. | Keep orthogonal to content tier and ViewMode. |
| `WP-1-Lens-ViewMode-v1` | baseline | Reuse NSFW default, explicit SFW toggle, hard-drop SFW projection, immutable raw/derived artifacts, and trace-visible filter. | Validated/done baseline; CKC projections must not mutate raw evidence. |
| `WP-1-Studio-Runtime-Visibility-v1` | partially folded | Preserve Studio jobs, workflow nodes, tool surfaces, DCC/operator projection, Flight Recorder linkage, Locus/WP linkage, and PostgreSQL-only state posture. | Needs boundary against DCC/projection ownership and no-SQLite tripwires. |
| `WP-1-Stage-Media-Artifact-Portability-v1` | separate dependency | Preserve portable manifests, bundle indexes, retention semantics, materialization anchors, and Stage/media portability. | Foundational dependency for media/DAM, intake, sidecars, ComfyUI receipts, exports, and recovery. |
| `WP-1-Stage-ASR-Transcript-Lineage-v1` | later CKC input | Preserve source media to ASR input to transcript artifact to searchable consumer lineage; source hashes, media probe facts, provenance, and timing anchors. | Media intelligence input; should not block the first character/media kernel unless transcript workflow is in slice scope. |
| `WP-1-ASR-Transcribe-Media-v1` | later CKC input | Preserve local-first ASR job, ffmpeg extraction, transcript artifacts, timing metadata, capability gating, and Flight Recorder events. | Depends on Workflow Engine, MEX runtime, and artifact foundations; cloud transcription must stay explicit opt-in. |
| `WP-1-Video-Archive-Loom-Integration-v1` | partially folded | Preserve LoomBlock wrappers, captions/transcript entities, local ASR fallback, tags, thumbnails/proxies, and transcript/caption Lens retrieval. | Overlaps media/DAM, search/tags, sidecars, and exports; use Loom as adjacent media-intelligence path. |
| `WP-1-Loom-MVP-v1` | baseline | Reuse LoomBlocks, LoomEdges, UUID identity, mentions/tags, backlinks, file import, SHA-256 dedup, previews, search, and FR-EVT-LOOM events. | Validated/done baseline; CKC extends identity/search semantics rather than replacing them. |
| `WP-1-Loom-Storage-Portability-v4` | operator-decision-needed | Preserve current-spec Loom storage proof, graph traversal, directional edges, metrics recomputation, and source-anchor durability under PostgreSQL-only reset. | Registry/task-board state is mixed; operator or registry cleanup must decide validated baseline versus active proof dependency. |
| `WP-1-Loom-Preview-VideoPosterFrames-v1` | partially folded | Preserve representative frame extraction, thumbnails, derived preview assets, LoomBlock links, ffmpeg/ffprobe gating, and Flight Recorder events. | Boundary needed against Photo Studio thumbnails and CKC media viewer thumbnails. |
| `WP-1-Media-Downloader-v1` | historical source | Preserve YouTube/Instagram/forum/generic archive intent, unified queue/progress, output roots, captions sidecars, deterministic naming, Bronze/Raw materialization, and Flight Recorder events. | Superseded by active v2; preserve detailed modes without reactivating v1. |
| `WP-1-Media-Downloader-v2` | separate dependency | Active downloader dependency for completed archive outputs, queue/progress, captions, capability gating, and evidence logging. | Use active packet path as implementation authority, not v1 stub. |
| `WP-1-Media-Downloader-Loom-Bridge-v1` | later CKC input | Preserve promotion from downloader artifacts to workspace assets/LoomBlocks, content-hash dedup, sidecars, preview enqueue, searchable captions, and promotion events. | Should consume artifact foundation before archive UI consolidation. |
| `WP-1-Product-Screenshot-Visual-Validation-v1` | baseline | Inherit programmatic full-window/panel/module capture, governed screenshot artifacts, CLI/API trigger, metadata, and Tauri/webview/native integration. | Superseded into Kernel002; CKC should inherit evidence path rather than reimplement. |
| `WP-1-Visual-Debugging-Loop-v1` | baseline | Inherit capture/compare/fix loop, baselines, visual diffs, validator routing, thresholds, and Tauri test mode. | Superseded into Kernel002; GUI CKC work needs visual evidence. |
| `WP-1-Artifact-System-Foundations-v1` | separate dependency | Preserve artifact store bootstrap, manifests, SHA-256, atomic Materialize API, retention, pinning, GC, and no random filesystem side effects. | Foundational for every media, sidecar, transcript, screenshot, export, and backup path. |
| `WP-1-Structured-Collaboration-Artifact-Family-v1` | separate dependency | Preserve versioned packet/summary/index/thread JSON/JSONL artifacts, compact summaries, project-agnostic envelopes, and Markdown migration guidance. | Governance substrate for no-context microtasks and source-backed packet projections. |
| `WP-1-Calendar-Lens-v3` | later CKC input | Preserve Lens query/view pattern ideas: source/date filters, title search, stable IDs, provenance, and optional timeline/Flight Recorder overlay. | Adjacent Lens pattern only; calendar implementation remains separate from CKC media/Atelier scope. |

## CKC Overlap View

| CKC Cluster | Handshake Overlap | Fold Decision | Preserved Requirements | Unresolved Gaps |
|---|---|---|---|---|
| Media viewer / DAM | Photo Studio thumbnails, Loom previews, Media Downloader outputs, Stage media artifacts, artifact foundations. | partially folded | Image-first browsing, galleries, fullscreen/slideshow, metadata, missing-media diagnostics, ratings, favorites, tags, provenance, deleted/archive visibility, OpenPose sidecar hiding. | Define ownership between Photo Studio, Loom, artifact store, and CKC media viewer; decide shared thumbnail/preview service. |
| Character sheets / templates | Atelier/Lens remediation, CKC character/sheet kernel, structured collaboration artifacts. | folded | Character profile plus sheet, byte-preserved user text, explicit/adult fields, public/internal ID split, append-only versions, templates, cloning, merge preview, selective apply. | Need Handshake-native schema and append-only version model with no silent rewrites. |
| Intake / inbox | Media Downloader, Stage import, CKC direct image ingress, role mailbox/inbox patterns. | partially folded | Persistent batches, accept/reject/pending, linked/loose mode, character/sheet/collection linkage, source preservation, no silent delete. | Reject CKC localhost intake authority; translate to typed endpoint or artifact proposal path with EventLedger lineage. |
| Collections / contact sheets | Loom collections/tags, Photo Studio/moodboard intent, CKC collection/contact-sheet artifacts. | folded | Cross-character curated sets, notes/tags, optional sheet-version links, slideshow, export, SVG/raster contact sheets, source IDs/hashes, layout metadata. | Need collection/contact-sheet manifest authority and export boundary. |
| Sidecars / versioning / recovery | Stage/media manifests, Loom portability, Lens ViewMode immutability, artifact foundations. | folded | OpenPose sidecars, append-only sheet versions, reverts-as-new-version, archive/restore, orphan manifests, event logs, entity revisions, idempotency checks. | Need artifact-row/ref model and recovery manifest shape. |
| PoseKit / OpenPose / identity | CKC PoseKit/OpenPose source, sidecar hiding, character identity, media provenance. | later CKC input | Body/face/hand arrays, zero-filled absent hands, yaw/pitch/roll, identity profiles, deterministic crops, landmarks, measurements, calibration-debt markers. | Keep full PoseKit calibration out of initial kernel/vertical slice unless operator promotes it. |
| ComfyUI lineage | Photo Studio recipes, CKC ComfyUI bridge, Stage/media artifact lineage, artifact foundations. | partially folded | Workflow/run receipts, history, prompt extraction, replay, stats, registered outputs, workflow/Pose tab replay paths, identity reference payload intent. | Reject localhost intake and direct CKC runtime bridge; define typed receipt and artifact proposal contracts. |
| Automation / manual / debug | Studio runtime visibility, structured collaboration artifacts, visual debugging, product screenshots. | folded | Typed command map, model manual, sessions, leases, heartbeats, command log, captures, no-focus/no-OS-input invariants, command/manual consistency tests. | Translate process-local sessions to PostgreSQL/EventLedger-backed leases and receipts. |
| Search / tags / similarity | Lens query patterns, Loom search/tags, ASR transcript search, Calendar Lens query pattern. | folded | Global search across sheets, notes, stories, moodboards, metadata, grouped results, snippets, jump targets, tag manager, backlinks, AI suggestions, palettes, duplicates, perceptual similarity. | Reject SQLite FTS5; map to Handshake search/index architecture and provenance-visible QueryPlan/RetrievalTrace. |
| Exports / backups / share packs | Artifact foundations, media downloader sidecars, structured collaboration artifacts, contact sheets. | folded | Empty templates, LLM packs, filled sheets, image sets, moodboards, collections, share packs, backups, web portfolios, contact sheets, presets, provenance, no-space names. | Define export/backup manifest checksums, retention, and no-data-loss recovery evidence. |

## Conflicts And Controls

1. SQLite/FTS5/runtime-DDL is source evidence only; control is absolute no-SQLite acceptance gate across runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temp harnesses, imports, exports, and product paths.
2. CKC Electron shell is source evidence only; control is Rust/Tauri command boundary with React projection after domain contracts.
3. CKC localhost intake is source evidence only; control is typed Handshake endpoint or artifact proposal path with EventLedger lineage.
4. Sparse Handshake contracts risk source-intent loss; control is one source-backed row per stub and explicit classification.
5. Validated and superseded stubs can be misread as executable backlog; control is baseline/historical/source-material classification before promotion.
6. Photo Studio, Loom, and media viewer thumbnails overlap; control is a shared preview/thumbnail boundary decision.
7. CKC convenience features can be dismissed as scope noise; control is fold/dependency/defer/conflict/operator-decision classification.
8. Product outputs under `.GOV` or machine-local paths would break governance split and portability; control is artifact-root-relative or operator-configured product paths only.

## Acceptance Gate Rows

| Gate | Required Evidence |
|---|---|
| Source coverage | One source-backed row exists for every listed stub and active dependency named above. |
| Overlap coverage | One overlap row exists for each CKC cluster: media/DAM, characters/templates, intake, collections/contact sheets, sidecars/versioning/recovery, PoseKit/OpenPose/identity, ComfyUI lineage, automation/manual/debug, search/tags/similarity, exports/backups/share packs. |
| Classification | Every source is classified as baseline, folded, partially folded, separate dependency, historical source, later CKC input, or operator-decision-needed. |
| No loss | No source intent is silently deleted, renamed away, or compressed into vague scope. |
| No SQLite | SQLite is rejected in all Handshake contexts, including tests, fixtures, mocks, examples, cache, compatibility, imports, exports, fallbacks, harnesses, and temp adapters. |
| Architecture translation | CKC implementation assumptions are translated into Handshake-native Rust/Tauri/PostgreSQL/EventLedger/artifact-store contracts. |
| Visual evidence | GUI-bearing future CKC/Atelier work inherits Kernel visual screenshot/diff evidence. |
| Research gate | CKC rebuild stubs remain deferred until consolidation, greenroom review, CKC research basis, and conflict classification are complete. |

## Microtask Candidates

1. Verify all source paths and active packet paths for the source rows.
2. Reconcile `WP-1-Loom-Storage-Portability-v4` task-board versus registry state.
3. Build source-hash inventory for every listed stub and active dependency.
4. Derive explicit acceptance for sparse `WP-1-Atelier-Lens-v2` Markdown intent.
5. Derive explicit acceptance for sparse `WP-1-Photo-Studio-v2` Markdown intent.
6. Draft media/DAM ownership boundary across Photo Studio, Loom, Stage/media, and CKC viewer.
7. Draft character/sheet/template schema preservation register.
8. Draft intake/inbox state machine and reject localhost intake authority.
9. Draft collections/contact-sheet manifest contract.
10. Draft sidecar/versioning/recovery artifact-row/ref contract.
11. Draft PoseKit/OpenPose deferred-input boundary.
12. Draft ComfyUI receipt and run-lineage contract without localhost runtime dependency.
13. Draft automation/manual/debug command catalog and lease/receipt model.
14. Draft search/tags/similarity behavior map without SQLite/FTS5.
15. Draft export/backup/share-pack manifest and checksum contract.
16. Create no-SQLite tripwire acceptance rows for future packets.
17. Create no-silent-rewrite/no-silent-delete acceptance rows.
18. Create visual evidence inheritance row for GUI work.
19. Create artifact-root/product-path hygiene row.
20. Prepare future CKC rebuild-stub gate checklist.
