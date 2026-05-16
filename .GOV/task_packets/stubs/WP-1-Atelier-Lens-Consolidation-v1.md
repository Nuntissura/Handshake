# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Atelier-Lens-Consolidation-v1

## STUB_METADATA
- WP_ID: WP-1-Atelier-Lens-Consolidation-v1
- BASE_WP_ID: WP-1-Atelier-Lens-Consolidation
- CREATED_AT: 2026-05-16T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening, WP-KERNEL-003-Sandbox-Validation-Promotion, WP-1-Artifact-System-Foundations, WP-1-Structured-Collaboration-Artifact-Family
- BUILD_ORDER_BLOCKS: WP-1-Atelier-Lens, WP-1-Photo-Studio, WP-1-Studio-Runtime-Visibility, future CKC rebuild stubs
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md
- ROADMAP_POINTER: FEAT-ATELIER-LENS; FEAT-PHOTO-STUDIO; PRIM-Moodboard; TOOL-COMFYUI; Stage/media lineage; Loom/archive media intelligence
- ROADMAP_ADD_COVERAGE: SPEC=v02.185; PHASE=7.6.3; LINES=12-end-of-file-appendices:155,889,4376,5728; 10-product-surfaces:4872,5134,5391,5797
- SOURCE_REFERENCE_DIR: .GOV/reference/ckc_atelier_lens_consolidation
- PREMATURE_CKC_STUBS_ARCHIVE: .GOV/task_packets/_archive/superseded/premature-ckc-stubs-20260516
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - `.GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md` FEAT-ATELIER-LENS / FEAT-PHOTO-STUDIO / PRIM-Moodboard / TOOL-COMFYUI
  - `.GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md` 10.10 Photo Studio, 10.10.4.5 Library/DAM Functions, 10.10.5.2 ComfyUI Integration Scope, 10.10.8.3 Moodboard Workflows
  - `.GOV/operator/docs_local/handshake-v2-kernel-reset-brief.md` PostgreSQL/EventLedger-only storage reset and no-SQLite rule

## INTENT (DRAFT)
- What: Consolidate all current Atelier/Lens-adjacent Handshake stubs into one preservation-first WP before any CKC rebuild stubs are created.
- Why: The current backlog has separate Atelier/Lens, Photo Studio, Lens, Studio runtime, media lineage, Loom/archive, artifact, and visual-debug intentions. Those must be folded without losing intent before CKC is rebuilt inside Handshake.
- Correction: This replaces the premature CKC Greenroom/Kernel/Vertical-Slice stubs. Greenroom is today's research/consolidation work, not a WP by itself. CKC rebuild stubs are deferred until greenroom output and CKC research are complete.
- Overlap rule: CKC is an evolved sibling expression of Atelier/Lens and prompt-diary intent, built outside Handshake because the same operator needs were urgent. Do not treat CKC as optional garnish, lower-priority inspiration, or a feature pile to cherry-pick after Atelier/Lens. Fold CKC into Atelier/Lens while preserving the existing Atelier/Lens paths.
- Convenience rule: CKC features created from unexpected need and daily convenience are valid requirement evidence. Classify them as folded, dependency, deferred, conflict, or operator-decision-needed; do not ignore them because they were not in the original Handshake stubs.

## SOURCE_STUBS_TO_CONSOLIDATE (DRAFT)
- `WP-1-Atelier-Lens-v2`: role claiming, SceneState, ConflictSet, and prior failed/gapped Atelier/Lens remediation intent.
- `WP-1-Photo-Studio-v2`: skeleton surface, thumbnails, recipes, and prior failed/gapped Photo Studio remediation intent.
- `WP-1-Atelier-Collaboration-Panel-v1`: role suggestions over current selection, multi-suggestion review, range-bounded Monaco/Docs patching, provenance logging, validator rejection of out-of-selection patches.
- `WP-1-Lens-Extraction-Tier-v1`: Tier0/Tier1/Tier2 model, Tier1 default, explicit override, requested/effective tier trace, invalid-tier validation.
- `WP-1-Lens-ViewMode-v1`: NSFW default, explicit SFW toggle, hard-drop SFW projection, immutable raw and derived artifacts, trace-visible ViewMode metadata filter.
- `WP-1-Studio-Runtime-Visibility-v1`: Studio jobs, workflow nodes, tool surfaces, DCC/operator projection, Flight Recorder linkage, Locus/task-board/WP linkage, PostgreSQL-only state.
- `WP-1-Stage-Media-Artifact-Portability-v1`: shared artifact portability across Stage session records, media capture sessions, debug bundle export, storage portability, bounded export anchors.
- `WP-1-Stage-ASR-Transcript-Lineage-v1`: source media -> ASR input -> transcript artifact -> searchable consumer lineage, stable hashes, media probe facts, session provenance, timing anchors.
- `WP-1-ASR-Transcribe-Media-v1`: governed ASR workflow job, ffmpeg extraction, transcript artifacts, timing metadata, capability gating, Flight Recorder events.
- `WP-1-Video-Archive-Loom-Integration-v1`: LoomBlock wrappers, captions/transcript entities, local ASR fallback, manual and suggested tags, thumbnail/proxy views, transcript/caption Lens retrieval.
- `WP-1-Loom-MVP-v1`: LoomBlocks, LoomEdges, UUID identity, mentions/tags, views, backlinks, file import, SHA-256 dedup, previews, search, FR-EVT-LOOM events.
- `WP-1-Loom-Storage-Portability-v4`: current-spec Loom storage proof, graph traversal, directional edges, metric recomputation, source-anchor durability under PostgreSQL-only reset.
- `WP-1-Loom-Preview-VideoPosterFrames-v1`: representative frame extraction, thumbnails, derived preview assets, LoomBlock linking, ffmpeg/ffprobe gating, Flight Recorder events.
- `WP-1-Media-Downloader-v2` and historical `WP-1-Media-Downloader-v1`: unified downloader queue/progress, output roots, captions sidecars, media crawl/download, deterministic naming, Bronze/Raw materialization, Flight Recorder events.
- `WP-1-Media-Downloader-Loom-Bridge-v1`: completed downloader artifacts to workspace assets/LoomBlocks, content-hash dedup, captions and metadata preservation, preview enqueue, searchable captions.
- `WP-1-Product-Screenshot-Visual-Validation-v1`: programmatic capture, governed artifact storage, CLI/API trigger, screenshot metadata, Tauri/webview/native integration.
- `WP-1-Visual-Debugging-Loop-v1`: post-commit screenshot capture, baseline comparison, diff artifacts, validator evidence routing, threshold configuration, Tauri app test mode.
- `WP-1-Artifact-System-Foundations-v1`: artifact store bootstrap, manifests, SHA-256, atomic Materialize API, retention/pinning/GC, no random filesystem side effects.
- `WP-1-Structured-Collaboration-Artifact-Family-v1`: JSON/JSONL packet, summary, index, thread artifacts; schemas; project-agnostic envelope; compact summaries; Markdown-to-structured migration.
- `WP-1-Calendar-Lens-v3`: adjacent Lens pattern source only; preserve query/view ideas while keeping calendar implementation separate.

## CKC_GREENROOM_AND_RESEARCH_INPUTS (DRAFT)
- CKC code/spec/taskboard greenroom artifacts in `.GOV/reference/ckc_atelier_lens_consolidation/` are reference input only for this consolidation WP.
- CKC source evidence must preserve product intent: media viewer, character sheet, media pipeline for ComfyUI, PoseKit/OpenPose sidecars, collections/contact sheets, exports, and automation.
- CKC source evidence must be compared against Atelier/Lens source intent as overlapping first-class product intent. Where CKC is more evolved, the consolidation must preserve the evolved behavior unless it conflicts with Handshake architecture or the operator explicitly rejects it.
- Extra CKC capabilities produced by real usage and convenience pressure are not scope noise. They are carried into the consolidation register with source anchors and a fold/defer/separate/conflict decision.
- CKC rebuild/kernel/vertical-slice stubs must not be created until:
  - the Atelier/Lens consolidation WP has classified preserved Handshake intent,
  - CKC greenroom output is reviewed as source evidence,
  - CKC research basis is recorded,
  - conflicts are resolved or explicitly deferred.
- The premature CKC Greenroom/Kernel/Vertical-Slice stubs are archived as correction evidence, not active backlog truth.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Build a single no-loss source register for all Atelier/Lens-adjacent stubs listed above.
  - Preserve every source stub's original intent, rationale, scope, constraints, unresolved work, and dependencies.
  - Classify each source stub as baseline, fully folded, partially folded, separate dependency, historical source, or later CKC input.
  - Build an overlap matrix between Atelier/Lens intent and CKC capability clusters so duplicate goals are merged instead of competing.
  - Build a CKC evolved-feature register for convenience-driven or unexpected-use features and require an explicit fold/defer/separate/conflict decision for each.
  - Produce a conflict matrix for UI-first legacy assumptions, SQLite/FTS5/source examples, Electron/localhost intake, artifact-path ownership, DCC/projection overlap, Loom/media overlap, and duplicate Photo Studio/Atelier responsibility.
  - Produce a consolidated microtask map for one future official Atelier/Lens WP, with small enough MTs for no-context model execution.
  - Keep CKC rebuild stubs deferred until greenroom and research are complete.
  - Preserve the PostgreSQL-only rule: SQLite is not accepted for runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths.
- OUT_OF_SCOPE:
  - Product runtime implementation.
  - Full GUI rebuild or dockable-window shell work.
  - Creating CKC rebuild/kernel/vertical-slice stubs now.
  - Claiming CKC parity.
  - Dropping any existing stub intent without explicit conflict/defer/supersede classification.

## ACCEPTANCE_CRITERIA (DRAFT)
- The official WP produced from this stub contains one source-backed row for every listed Atelier/Lens-adjacent source stub.
- The official WP contains one source-backed overlap row for each CKC capability cluster that overlaps Atelier/Lens, Photo Studio, Studio runtime, Loom/media/archive, artifact, or visual-debugging intent.
- Every CKC convenience feature extracted from code/spec/taskboard is classified as folded, dependency, deferred, conflict, or operator-decision-needed.
- No listed source stub intent is silently deleted, renamed away, or compressed into vague wording.
- Every conflict between old and new scope has one of: layered resolution, parallel/dependency split, explicit defer, or operator-decision-needed.
- CKC material remains reference/research input until a later CKC rebuild stub is created after greenroom and research.
- SQLite is rejected in every Handshake context: runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- The WP outputs enough detail for later CKC rebuild stubs to be created without rereading every legacy Atelier/Lens stub from scratch.

## MICRO_TASK_BUCKETS (DRAFT)
1. Register source files and hashes for every listed stub.
2. Extract original intent and rationale from each source stub.
3. Extract scope, out-of-scope, dependencies, blockers, and risks from each source stub.
4. Preserve sparse Markdown-only intent from `WP-1-Atelier-Lens-v2`.
5. Preserve sparse Markdown-only intent from `WP-1-Photo-Studio-v2`.
6. Fold Collaboration Panel role suggestion and bounded patch semantics.
7. Fold Lens Extraction Tier model and trace requirements.
8. Fold Lens ViewMode SFW/NSFW projection and immutable artifact requirements.
9. Fold Studio runtime visibility and projection requirements.
10. Fold Stage media artifact portability requirements.
11. Fold Stage ASR transcript lineage requirements.
12. Fold ASR transcribe workflow requirements.
13. Fold Video Archive Loom integration requirements.
14. Fold Loom MVP graph/search/view requirements.
15. Fold Loom storage proof requirements under PostgreSQL-only reset.
16. Fold Loom preview/video poster requirements.
17. Fold Media Downloader v2 validated baseline and v1 historical intent.
18. Fold Media Downloader to Loom bridge requirements.
19. Fold screenshot capture and visual validation requirements.
20. Fold visual debugging loop requirements.
21. Fold artifact foundation requirements.
22. Fold structured collaboration artifact requirements.
23. Classify Calendar Lens as adjacent pattern source, not folded implementation.
24. Create the old/new UI responsibility conflict matrix.
25. Create the Photo Studio versus Atelier/Lens responsibility boundary.
26. Create the Studio runtime versus DCC/projection boundary.
27. Create the Loom/media/archive overlap boundary.
28. Create the artifact store and media file authority boundary.
29. Create the ComfyUI/PoseKit deferred-input boundary.
30. Create the CKC source evidence boundary.
31. Create the no-SQLite tripwire row.
32. Create the PostgreSQL/EventLedger lineage requirement row.
33. Create the visual-debug verification requirement row.
34. Create the no-data-loss and no-silent-delete requirement row.
35. Create the no-silent-rewrite/versioning requirement row.
36. Create the provenance and artifact-manifest requirement row.
37. Create the model/no-context manual requirement row.
38. Create the future CKC research gate.
39. Create the future CKC rebuild stub creation gate.
40. Create the source-stub supersession/retention plan.
41. Create the Atelier/Lens versus CKC overlap matrix.
42. Create the CKC evolved-feature and convenience-driven requirement register.
43. Classify every CKC extra feature as folded, dependency, deferred, conflict, or operator-decision-needed.
44. Update taskboard and traceability registry after activation.
45. Regenerate stub contracts and derived records.
46. Run packet truth checks.
47. Record operator decisions and unresolved conflicts.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Kernel002/Kernel003 direction for write boxes, promotion, validation, and sandbox evidence.
- Depends on: Artifact System Foundations and Structured Collaboration Artifact Family for durable media/projection records.
- Blocks: reactivation of `WP-1-Atelier-Lens-v2`, `WP-1-Photo-Studio-v2`, `WP-1-Studio-Runtime-Visibility-v1`, and future CKC rebuild stubs until the consolidation contract is accepted.

## RISKS / UNKNOWNs (DRAFT)
- Risk: consolidation compresses away source intent. Control: one source-backed row per source stub and explicit fold/defer/separate classification.
- Risk: CKC gets treated as secondary because it was built outside Handshake. Control: require an overlap matrix and evolved-feature register before any CKC rebuild stub is authored.
- Risk: CKC urgency shortcuts the Handshake-native architecture. Control: CKC rebuild stubs are gated on greenroom plus research and must inherit PostgreSQL/EventLedger/ArtifactStore boundaries.
- Risk: UI shell desire pollutes backend/domain planning. Control: this WP records future UI needs but does not implement the dockable shell.
- Risk: media/Loom/Photo Studio responsibilities blur. Control: produce explicit ownership boundary rows before runtime work starts.
- Risk: old SQLite/FTS5 examples leak back into tests. Control: require no-SQLite tripwires in acceptance and future microtasks.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body and Appendix feature/tool/primitive rows.
- [ ] Produce the in-chat Technical Refinement Block.
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Atelier-Lens-Consolidation-v1.md`.
- [ ] Create official task packet via `just create-task-packet WP-1-Atelier-Lens-Consolidation-v1`.
- [ ] Copy source-backed scope from this stub and `.GOV/reference/ckc_atelier_lens_consolidation/`.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
- [ ] Do not create CKC rebuild/kernel/vertical-slice stubs until this consolidation, CKC greenroom, and CKC research are complete.
