<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.refinement_contract@1 source_file=.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json source_hash=14141630d1114cdb projection_hash=6bf648c4cf8ec555 generated_at_utc=2026-05-16T03:39:00.000Z generator=.GOV/roles_shared/scripts/wp/atelier-lens-consolidation-packet-generator.mjs -->
# WP-1-Atelier-Lens-Consolidation-v1 Refinement: Atelier/Lens Consolidation and CKC Fold-In

## METADATA
- WP_ID: WP-1-Atelier-Lens-Consolidation-v1
- BASE_WP_ID: WP-1-Atelier-Lens-Consolidation
- STATUS: READY_FOR_DEV
- USER_SIGNATURE: ilja160520260339
- PACKET_FORMAT_VERSION: 2026-04-06
- SOURCE_PACKET: .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/packet.md
- SOURCE_REFERENCE_ROOT: .GOV/reference/ckc_atelier_lens_consolidation

## OPERATOR_INTENT
The operator wants momentum on the original prompt-diary to Atelier/Lens goal: a media viewer, character sheet, media pipeline, and production companion folded into Handshake instead of maintained as a separate CKC app. Existing Atelier/Lens stubs must not be discarded. CKC is more evolved because it was built from unexpected need and convenience, so those extra behaviors must be preserved, classified, and translated into Handshake.

## REFINEMENT_STANCE
- Preserve-first consolidation: every existing Atelier/Lens-adjacent stub remains source material unless the operator explicitly supersedes it.
- CKC fold-in: CKC features become evidence for Atelier/Lens, Photo Studio, Studio runtime, media lineage, artifact, and visual-debug surfaces.
- No premature CKC rebuild stubs: downstream CKC implementation packets remain deferred until this consolidation and CKC research basis are complete.
- SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths. PostgreSQL/EventLedger/ArtifactStore are the only accepted storage and evidence direction.

## SOURCE_STUB_PRESERVATION
  - WP-1-Atelier-Lens-v2: intent=Additive remediation for failed or gapped Atelier Lens work.; handling=source_material_for_new_ckc_runway_stub; risks=Sparse machine contract; meaningful source is the Markdown projection.; Prior packet was not read in this task.
  - WP-1-Photo-Studio-v2: intent=Additive remediation for failed or gapped Photo Studio work.; handling=source_material_for_new_ckc_runway_stub; risks=Sparse machine contract; possible overlap with Loom poster frame and preview work.
  - WP-1-Atelier-Collaboration-Panel-v1: intent=Implement a selection-scoped Atelier Collaboration Panel in editor surfaces.; handling=separate_validated_baseline_to_reuse; risks=Editor range off-by-one errors; Boundary normalization loopholes
  - WP-1-Lens-Extraction-Tier-v1: intent=Implement LensExtractionTier as a first-class runtime and planning input.; handling=source_material_for_new_ckc_runway_stub; risks=Confusion with content_tier; Implicit Tier2 creep through heuristics
  - WP-1-Lens-ViewMode-v1: intent=Implement ViewMode UI and enforcement for Lens outputs.; handling=separate_validated_baseline_to_reuse; risks=Inconsistent enforcement across surfaces; Accidental stored artifact mutation
  - WP-1-Stage-Media-Artifact-Portability-v1: intent=Unify Stage capture/import sessions and Media Downloader outputs under portable artifact manifest, bundle-index, and retention semantics.; handling=source_material_for_ckc_foundation_wp; risks=Incompatible Stage/media manifests; Capture-session provenance omitted from general portability rules
  - WP-1-Stage-ASR-Transcript-Lineage-v1: intent=Define backend lineage from Stage-captured/imported media to governed ASR transcript artifacts.; handling=source_material_for_ckc_foundation_and_media_wp; risks=Separate identity fields across Stage/media/ASR; Timing anchors or provenance lost while text survives
  - WP-1-Studio-Runtime-Visibility-v1: intent=Make Studio and Design Studio surfaces explicit runtime citizens.; handling=source_material_for_ckc_atelier_lens_studio_wp; risks=Fragmented UI panels; Hidden side effects; Accidental SQLite reintroduction
  - WP-1-ASR-Transcribe-Media-v1: intent=Implement local-first ASR transcription for audio/video media.; handling=source_material_for_ckc_media_intelligence_wp; risks=Long-video performance and memory; Transcript size and indexing cost
  - WP-1-Video-Archive-Loom-Integration-v1: intent=Turn archived/imported video files into Loom library objects with searchable transcripts, captions sidecars, and tags/mentions that compose with Lens/Atelier.; handling=source_material_for_ckc_media_intelligence_wp; risks=Transcript indexing cost; Captions versus ASR duplicates; Privacy and cloud-send confusion
  - WP-1-Loom-MVP-v1: intent=Deliver the Phase 1 Loom MVP local-first library surface.; handling=separate_validated_baseline_to_reuse; risks=Anchor drift; Dedup identity errors; Preview jobs degrade UI responsiveness
  - WP-1-Loom-Storage-Portability-v4: intent=Re-open Loom portability as a narrow remediation/proof pass separating real portability evidence from narrative closure.; handling=separate_validated_or_proof_baseline; clarify_registry_state_later; risks=v3 mixed concrete storage work with broad closure claims; Dual-backend proof can expand too far
  - WP-1-Loom-Preview-VideoPosterFrames-v1: intent=Support Tier-1 previews for video assets by generating deterministic poster-frame thumbnails as a background mechanical job.; handling=source_material_for_ckc_media_intelligence_wp; risks=ffmpeg-version determinism; Large-video performance and cancellation
  - WP-1-Media-Downloader-v1: intent=Batch archive web media into local-first resumable ingest jobs with capability gating and evidence logging.; handling=historical_source_material; use_active_v2_as_dependency; risks=Platform policy changes; Fragile parsing; Large storage; Partial downloads; Caption variability; Forum variance
  - WP-1-Media-Downloader-Loom-Bridge-v1: intent=Make Media Downloader outputs promotable into Loom as LoomBlocks.; handling=source_material_for_ckc_media_intelligence_wp; risks=Artifact versus asset duplication; Large-file non-streaming imports; Caption language and track variability
  - WP-1-Product-Screenshot-Visual-Validation-v1: intent=Build product-integrated screenshot capture for full app window, panels, and module-level views.; handling=superseded_platform_requirement_inherit_via_kernel; risks=Panel granularity may need DOM capture; Headless/mock UI mismatch; File size and retention; Native versus DOM capture uncertainty
  - WP-1-Visual-Debugging-Loop-v1: intent=Implement generate-capture-compare-fix visual debugging loop for GUI work packets.; handling=superseded_platform_requirement_inherit_via_kernel; risks=Pixel false positives; Headless rendering mismatch; Threshold tuning noise
  - WP-1-Calendar-Lens-v3: intent=Implement Calendar Lens as first-class UI and API workflow.; handling=adjacent_lens_pattern_source_keep_separate; risks=Timezone and DST; Event privacy and accidental indexing
  - WP-1-Artifact-System-Foundations-v1: intent=Ensure artifact system foundations across exports and jobs.; handling=foundational_dependency_keep_separate; risks=Competing materialize implementations; Retention/GC data-loss bugs
  - WP-1-Structured-Collaboration-Artifact-Family-v1: intent=Define canonical structured collaboration artifacts for work packets, microtasks, task board projections, and role mailbox exports.; handling=foundational_governance_substrate_keep_separate; risks=Overfitting to repository-centric work; Summary drift; Premature migration complexity

## PRESERVED_INTENT_GROUPS
  - Atelier/Lens remediation must preserve role claiming, SceneState, ConflictSet, selection-scoped patching, role suggestion provenance, extraction tier, and ViewMode projection law.
  - Photo Studio remediation must preserve skeleton surface, thumbnails, and recipes.
  - Studio runtime must become traceable across jobs, tools, DCC/operator views, Flight Recorder, Locus, and storage backends.
  - Stage/media artifacts must use portable manifests, bundle indexes, source hashes, retention evidence, and replayable provenance.
  - ASR and transcript lineage must preserve timing anchors, source media identity, media probe facts, local-first execution, and searchable transcript linkage.
  - Loom/video archive must preserve LoomBlock identity, captions sidecars, transcripts, poster frames, tags, mentions, search facets, and large-library usability.
  - Validated and superseded stubs must remain source evidence or inherited requirements, not be silently dropped during CKC consolidation.

## CONFLICTS_AND_LAYERED_HANDLING
  - CONFLICT-001: Atelier-Lens-v2 and Photo-Studio-v2 have sparse machine contracts but meaningful Markdown projections.; risk=CKC drops original gap intent because draft_scope is empty in JSON.; handling=Preserve Markdown gap list as source material and create fresh CKC refinements with explicit acceptance.
  - CONFLICT-002: Validated work still has stub artifacts under the stubs directory.; risk=Agents may reopen completed work or treat historical stubs as executable.; handling=Keep validated work as baselines; CKC only adds deltas.
  - CONFLICT-003: Superseded visual stubs are still needed as CKC validation requirements.; risk=CKC GUI work loses screenshot and visual regression evidence.; handling=Inherit through Kernel002/Kernel003 visual evidence paths; do not reimplement unless coverage is missing.
  - CONFLICT-004: Stage/media foundation is high priority but marked blocked by downstream-facing packets.; risk=Downstream bridge/archive work invents manifest semantics before backend portability is settled.; handling=Create CKC foundation WP first, then consume it from bridge/archive WP.
  - CONFLICT-005: ViewMode projection can conflict with raw evidence preservation.; risk=SFW projection mutates or hides raw evidence in a way that breaks replay/audit.; handling=ViewMode is projection-only; raw artifacts and evidence refs remain immutable.
  - CONFLICT-006: Media Downloader v1 is superseded by v2 but contains detailed source-mode requirements.; risk=Instagram, forum, captions, progress, and output layout requirements are lost.; handling=Use active v2 as dependency while preserving v1 requirements as historical source material.
  - CONFLICT-007: Activation checklist paths reference old records locations.; risk=Future activation edits target stale paths.; handling=Normalize future packet checklists to .GOV/roles_shared/records paths without editing historical stubs.

## CKC_CAPABILITY_CLUSTERS
  - media_viewer_dam: image-first portfolio browsing,character galleries,slideshow,fullscreen,metadata,missing-media states,repair flows,ratings,favorites,tags,notes,provenance,deleted/archive visibility,OpenPose sidecar hiding
  - character_sheets: character profile plus sheet core shape,byte-preserved user text,explicit/adult fields first-class,protected public IDs,internal IDs,append-only versions,templates,cloning,merge preview,selective apply
  - inbox_intake: direct image ingress,persistent batches,accept/reject/pending,linked/loose mode,character/sheet/collection linkage,source preservation,no silent deletes
  - collections_contact_sheets: cross-character curated image sets,collection notes/tags,optional character/sheet-version links,slideshow,export,SVG contact sheet with manifest,source IDs/hashes,tags,layout metadata,raster contact-sheet follow-up
  - sidecars_versioning: OpenPose PNG/JSON sidecars,append-only sheet versions,reverts-as-new-version,archive/restore,orphan manifests,event logs,entity revisions,compatibility fixtures,migrations,idempotency checks
  - posekit_openpose: blank/single-photo/collection workbench modes,body-18,face-70,left/right hand-21 arrays,zero-filled absent hands,yaw/pitch/roll,identity profiles,deterministic crops,landmarks,measurements,blocked calibration/history debt
  - comfyui_bridge: local workflow/run lineage,history,prompt extraction,replay,stats,registered outputs,workflow/Pose tab replay paths,identity reference payload intent,prompt-response matrix later scope
  - automation_debug_manual: typed command map,in-app manual,automation sessions,leases,heartbeats,command log,captures,background/no-focus/no-OS-input invariants,visual captures,command/manual consistency tests
  - search_tags_similarity: global search,grouped results,snippets,jump targets,tag manager,links/backlinks,AI suggestions,palettes,duplicates,perceptual similarity
  - exports: empty templates,LLM packs,filled sheets,image sets,moodboards,collections,share packs,backups,web portfolios,contact sheets,batch exports,field/section selections,presets,no-space names,provenance

## HANDSHAKE_STUB_GOALS
  - WP-1-Atelier-Lens-v2: role claiming,SceneState,ConflictSet
  - WP-1-Photo-Studio-v2: skeleton surface,thumbnails,recipes
  - WP-1-Atelier-Collaboration-Panel-v1: selection-scoped role suggestions,range-bounded patching,provenance
  - WP-1-Lens-Extraction-Tier-v1: Tier0/Tier1/Tier2,Tier1 default,explicit override,requested/effective trace,validation
  - WP-1-Lens-ViewMode-v1: NSFW default,explicit SFW toggle,SFW hard-drop projection,immutable raw artifacts,trace-visible filter
  - WP-1-Stage-Media-Artifact-Portability-v1: portable artifact manifests,bundle index,retention evidence,storage portability,replayable provenance
  - WP-1-Stage-ASR-Transcript-Lineage-v1: source hashes,media probe facts,capture/session provenance,timing anchors,searchable transcript linkage
  - WP-1-Studio-Runtime-Visibility-v1: Studio job/tool/workflow mapping,DCC/operator projection,Flight Recorder linkage,Locus/WP linkage,storage posture

## OVERLAP_ROWS
- OVR-001: area=Character sheets and Atelier identity; CKC=app/backend/db.js, app/backend/library.js, src/ui/views/CharacterView.tsx; Handshake=WP-1-Atelier-Lens-v2, WP-1-Photo-Studio-v2, WP-1-Atelier-Collaboration-Panel-v1, FEAT-ATELIER-LENS; decision=fold_into_atelier_lens_core; preserve=stable public/internal IDs,protected fields,append-only sheet versions,selective merge/apply,byte-preserved user text,role/provenance rules
- OVR-002: area=Media viewer / DAM / Photo Studio; CKC=app/backend/db.js, app/backend/library.js, src/ui/views/LibraryView.tsx, src/ui/components/MediaPane.tsx, src/ui/views/CharacterView.tsx; Handshake=WP-1-Photo-Studio-v2, WP-1-Stage-Media-Artifact-Portability-v1, Loom/media downloader stubs; decision=fold_with_artifact_store_dependency; preserve=image-first browsing,thumbnails,metadata,provenance,missing-file diagnostics,archive/restore,sidecar hiding
- OVR-003: area=Intake / Inbox / pending review; CKC=app/backend/library.js, src/ui/views/IntakeSorterView.tsx, CKC taskboard WP-0094, CKC taskboard WP-0124, CKC taskboard WP-0125, CKC taskboard WP-0126; Handshake=WP-1-Stage-Media-Artifact-Portability-v1, WP-1-Media-Downloader-v2, WP-1-Media-Downloader-Loom-Bridge-v1; decision=fold_as_atelier_intake_subsystem; preserve=persistent batches,accept/reject/pending,loose/linked modes,source preservation,character/sheet/collection linkage,resume after route switch/restart
- OVR-004: area=Collections and contact sheets; CKC=app/backend/db.js, app/backend/library.js, CKC taskboard WP-0128, CKC taskboard WP-0129, CKC taskboard WP-0136; Handshake=WP-1-Loom-MVP-v1, WP-1-Photo-Studio-v2, WP-1-Artifact-System-Foundations-v1; decision=fold_with_raster_export_deferred; preserve=notes/tags,optional character/sheet-version links,SVG/contact-sheet manifests,source IDs/hashes,layout metadata,planned PNG/JPG path
- OVR-005: area=Sidecars, versioning, recovery; CKC=app/backend/db.js, app/backend/library.js, CKC taskboard WP-0130, CKC taskboard WP-0132, CKC taskboard WP-0134, CKC taskboard WP-0135; Handshake=WP-1-Lens-ViewMode-v1, WP-1-Artifact-System-Foundations-v1, WP-1-Stage-Media-Artifact-Portability-v1; decision=fold_with_artifact_and_event_lineage; preserve=sidecar visibility projection,archive/restore,optimistic revision checks,append-only events,deletion preview,no silent source deletion
- OVR-006: area=PoseKit / OpenPose / identity; CKC=src/posekit/core.mjs, src/posekit/poseDetection.worker.ts, app/backend/library.js, CKC taskboard WP-0107..WP-0115, CKC taskboard WP-0131, CKC taskboard WP-0132, CKC taskboard WP-0133; Handshake=FEAT-ATELIER-LENS, FEAT-PHOTO-STUDIO, TOOL-COMFYUI; decision=fold_as_deferred_ckc_atelier_feature_family; preserve=blank/single/collection workbench modes,identity profile lineage,deterministic OpenPose sidecars,multi-rig tabs,blocked calibration/history debt
- OVR-007: area=ComfyUI workflow lineage; CKC=comfyui_node/castkit_codex_bridge.py, app/backend/intakeServer.js, app/backend/comfyuiClient.js, app/backend/library.js, CKC taskboard WP-0109; Handshake=TOOL-COMFYUI, WP-1-Photo-Studio-v2, WP-1-Studio-Runtime-Visibility-v1; decision=fold_intent_reject_localhost_authority; preserve=workflow receipts,prompt extraction,replay,stats,identity reference payloads,output image registration,non-fatal bridge failure posture
- OVR-008: area=Search, tags, links, similarity; CKC=app/backend/library.js, app/backend/db.js, app/backend/dhash.js, app/backend/palette.js, CKC taskboard WP-0016/WP-0054/WP-0059/WP-0066/WP-0067/WP-0083/WP-0089; Handshake=WP-1-Lens-Extraction-Tier-v1, WP-1-Lens-ViewMode-v1, WP-1-Loom-MVP-v1; decision=fold_into_lens_projection_search_layer; preserve=snippets,jump targets,tag manager,saved searches,backlinks,palettes,dHash similarity,AI tag suggestions
- OVR-009: area=Docs, stories, moodboards, prompt diary intent; CKC=app/backend/db.js, app/backend/library.js, src/ui/components/MoodboardCanvas.tsx, CKC taskboard WP-0005/WP-0071..WP-0082; Handshake=prompt diaries, FEAT-ATELIER-LENS, WP-1-Loom-MVP-v1, WP-1-Structured-Collaboration-Artifact-Family-v1; decision=fold_into_atelier_lens_creative_planning_layer; preserve=docs inside character workflow,moodboard structured JSON,layers/folders,corkboard/outliner,links/backlinks,exports,text preservation
- OVR-010: area=Automation, model manual, visual diagnostics; CKC=app/backend/automationManual.js, app/backend/automationCommandMap.js, app/backend/automationControl.js, app/backend/automationStealth.js, CKC taskboard WP-0093/WP-0095/WP-0099/WP-0122/WP-0137; Handshake=GLOBAL-BUILD-MANUAL, GLOBAL-BUILD-DIAG, WP-1-Visual-Debugging-Loop-v1, WP-1-Studio-Runtime-Visibility-v1; decision=fold_as_model_operation_requirement; preserve=command catalog,sessions/leases,heartbeats,command log,renderer state,captures,no OS-level input,no focus stealing,manual/command consistency tests
- OVR-011: area=Exports, backups, share packs, web portfolio; CKC=app/backend/library.js, app/backend/backup.js, app/templates/web-portfolio, CKC taskboard WP-0006/WP-0063/WP-0087/WP-0105/WP-0106; Handshake=WP-1-Artifact-System-Foundations-v1, WP-1-Stage-Media-Artifact-Portability-v1, WP-1-Structured-Collaboration-Artifact-Family-v1; decision=fold_through_artifact_export_job_layer; preserve=no-space names,safe subsets,LLM packs,manifests,backup version guard,orphan adoption,checksums,offline portfolio intent
- OVR-012: area=Parallel editing / event log / revisions; CKC=app/backend/db.js, app/backend/library.js, app/backend/automationManual.js, CKC taskboard WP-0134; Handshake=WP-KERNEL-001-Event-Ledger-Session-Broker-v1, WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1, WP-KERNEL-003-Sandbox-Validation-Promotion-v1; decision=fold_into_eventledger_crdt_boundary; preserve=PostgreSQL source of truth,sessions/leases,EventLedger events,optimistic revisions,CRDT only for safe merge shapes

## EVOLVED_FEATURE_ROWS
- EVOL-001: feature=Stable public character IDs separate from internal IDs; decision=fold; why=Operator-facing character identity needs stable labels without leaking storage keys.
- EVOL-002: feature=Typed character sheet parser with union and block-list fields; decision=fold; why=CKC solved real sheet editing pain with typed fields, descriptor fallbacks, score normalization, and nested block editing.
- EVOL-003: feature=Append-only sheet versions with selective apply/revert; decision=fold; why=Prevents data loss when models or imports update sheets.
- EVOL-004: feature=Bulk character operations; decision=fold; why=Daily library work requires multi-select, bulk tags/fields, batch exports, trash, and restore.
- EVOL-005: feature=Persistent Intake batches; decision=fold; why=CKC moved beyond one-off import into recoverable review work.
- EVOL-006: feature=Contact sheets as artifacts; decision=fold; why=Contact sheets are useful for comparing generated or curated media and for handoff.
- EVOL-007: feature=OpenPose sidecars hidden from normal galleries; decision=fold; why=Sidecars are production artifacts but clutter normal viewing unless projected intentionally.
- EVOL-008: feature=PoseKit blank/single/collection workbench contexts; decision=fold_deferred_feature_family; why=Operator needs PoseKit to operate on collections and individual photos, not only a single character page.
- EVOL-009: feature=Body-18 / face-70 / hand-21 rig contract; decision=fold_deferred_feature_family; why=CKC already hardened an OpenPose-compatible pose representation useful for ComfyUI and image production.
- EVOL-010: feature=Quaternion-backed yaw/pitch/roll head pose; decision=fold; why=Fine control over generated character pose was an evolved production need.
- EVOL-011: feature=Identity profiles for face-reference workflows; decision=fold; why=Stable identity references are directly tied to character consistency and ComfyUI workflows.
- EVOL-012: feature=Multi-rig workspace tabs; decision=defer_after_kernel; why=Real pose workflow needs multiple drafts open without deleting stored rigs.
- EVOL-013: feature=ComfyUI output registration and replay; decision=fold_intent_adapt_runtime; why=CKC already closed the loop from generation output back into character/media lineage.
- EVOL-014: feature=Workflow spec registry and image-sourcing adapter; decision=fold; why=External generation/sourcing task output needs versioned ingestion and idempotency.
- EVOL-015: feature=Identity-decoupled media filenames; decision=fold; why=Prevents character names or sensitive sheet fields from leaking into paths or events.
- EVOL-016: feature=Global search with snippets and jump targets; decision=fold; why=Fast retrieval across sheets, notes, images, and moodboards is core Lens behavior.
- EVOL-017: feature=Tag manager, saved searches, palettes, dHash similarity; decision=fold; why=Convenience features became library-scale navigation requirements.
- EVOL-018: feature=Moodboard canvas inside character workflow; decision=fold; why=Prompt diaries and visual planning need more than static notes.
- EVOL-019: feature=Built-in model manual and command map; decision=fold; why=Lets no-context models operate the product.
- EVOL-020: feature=Sessions, leases, heartbeats, command logs; decision=fold_with_stronger_durability; why=CKC evolved toward multi-agent operation before Handshake's kernel reset.
- EVOL-021: feature=Non-focus-stealing automation and visual capture; decision=fold; why=Model-driven GUI verification must be quiet and reproducible.
- EVOL-022: feature=Filesystem health and recoverable deletion; decision=fold; why=Real media libraries drift; deletion must be reversible and explainable.
- EVOL-023: feature=Backup version traceability and orphan adoption; decision=fold; why=Recovery became a first-class operator need.
- EVOL-024: feature=Web portfolio and share pack exports; decision=fold; why=Operator needs portable handoff outputs.
- EVOL-025: feature=Hybrid CRDT/event-log policy; decision=fold_into_kernel_boundary; why=CKC discovered the right boundary: not everything needs CRDT, but parallel edits need receipts and revisions.
- EVOL-026: feature=Blocked PoseKit calibration/history debt; decision=defer_and_preserve; why=Unfinished work is still requirement evidence: draggable calibration, missing-marker placement, 3D/live split editing, forked history.

## TRANSLATION_MATRIX
  - atelier_core
  - atelier_sheet
  - atelier_media
  - atelier_intake
  - atelier_collections
  - atelier_sidecars
  - atelier_posekit
  - atelier_comfy
  - atelier_search
  - atelier_exports
  - atelier_automation
  - kernel_event_bridge

## RUNTIME_REJECTIONS
  - SQLite in any form, including runtime/tests/fixtures/mocks/examples/fallbacks/cache/compatibility adapters/temporary harnesses/imports/exports/product paths
  - Electron IPC as runtime authority
  - CKC localhost intake as authority
  - .GOV as product output root
  - CKC product namespace in Handshake runtime
- SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths. PostgreSQL/EventLedger/ArtifactStore are the only accepted storage and evidence direction.

## RESEARCH_BASIS
  - ComfyUI SaveImage node documentation: https://docs.comfy.org/built-in-nodes/SaveImage -> Image pipeline receipts need explicit output file, filename prefix, subfolder, and workflow prompt context; Handshake should map this into ArtifactStore/EventLedger receipts.
  - ComfyUI V3 custom node migration documentation: https://docs.comfy.org/custom-nodes/v3_migration -> ComfyUI integration must tolerate node API evolution and should use an adapter boundary instead of embedding CKC assumptions.
  - ComfyUI-SaveImageWithMetaData GitHub project: https://github.com/nkchocoai/ComfyUI-SaveImageWithMetaData -> Community workflows preserve generation metadata in output images; Handshake should preserve prompt/workflow metadata as explicit artifact provenance.
  - OpenPose output documentation: https://github.com/CMU-Perceptual-Computing-Lab/openpose/blob/master/doc/02_output.md -> Pose sidecars need stable schemas for body, face, and hand keypoints; CKC PoseKit should become a Handshake pose artifact contract.
  - Playwright visual comparisons: https://playwright.dev/docs/test-snapshots -> Visual/debug surfaces should be reproducible through deterministic snapshots and explicit evidence paths for model review.
  - Automerge local-first CRDT overview: https://automerge.org/ -> Parallel model edits should flow through CRDT-friendly document and artifact boundaries instead of single-owner desktop state.
  - PostgreSQL full text search documentation: https://www.postgresql.org/docs/17/textsearch.html -> Search/tag/similarity features can start PostgreSQL-first with full-text search before specialized vector or media indexes.
  - Tauri WebviewWindow API reference: https://tauri.app/reference/javascript/api/namespacewebviewwindow/ -> Window/tab/workspace behavior should translate to Handshake/Tauri-controlled surfaces rather than CKC Electron IPC authority.

## REUSE_OPPORTUNITIES
- Handshake PostgreSQL/EventLedger/ArtifactStore boundaries replace CKC SQLite, file-local persistence, and localhost authority.
- Handshake CRDT/workspace surfaces carry parallel model editing and tab/window placement needs without copying CKC desktop state.
- Handshake visual-debug and screenshot validation work can absorb CKC media review and contact-sheet review evidence.
- Existing Photo Studio, Atelier/Lens, Lens ViewMode, Lens Extraction Tier, Stage media artifact portability, Loom archive, and artifact-system packets remain the runway for implementation.

## REJECTED_OPTIONS
- Keep CKC as a separate source of product authority: rejected because the operator wants CKC folded into Handshake Atelier/Lens.
- Create CKC rebuild stubs before consolidation: rejected because the operator made consolidation first the current task.
- Use SQLite for runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths: rejected absolutely.
- Copy Electron IPC, localhost intake, or CKC namespace authority into Handshake: rejected; translate to Handshake stack boundaries.

## RED_TEAM
- Risk: CKC convenience features are treated as optional extras and lost. Mitigation: evolved feature register is acceptance evidence, and MTs classify every extra as folded, dependency, deferred, conflict, or operator-decision-needed.
- Risk: Atelier/Lens stub intent is overwritten by CKC scope. Mitigation: preservation map is a required source and MT-027 through MT-038 preserve source stub intent before rebuild work.
- Risk: SQLite sneaks back through tests, fixtures, caches, import/export compatibility, or temporary harnesses. Mitigation: absolute rejection is a clause row, acceptance criterion, and MT-039.
- Risk: governance and product code blur. Mitigation: this packet is governance-only; future implementation packets must write product code under Handshake surfaces and keep .GOV for repo governance.
- Risk: future coders implement UI first. Mitigation: packet constrains the next step to language/tech-stack translation and source-backed CKC fold-in, not GUI redesign.

## MICROTASK_GROUPS
  - MT-001: Inventory - Inventory CKC package/runtime dependencies
  - MT-002: Inventory - Inventory CKC backend service files
  - MT-003: Inventory - Inventory CKC UI views and reusable behavior
  - MT-004: Inventory - Inventory CKC PoseKit files
  - MT-005: Inventory - Inventory CKC ComfyUI bridge files
  - MT-006: Inventory - Inventory CKC test suite by behavior area
  - MT-007: Inventory - Inventory CKC spec headings and requirement sections
  - MT-008: Inventory - Inventory CKC taskboard statuses
  - MT-009: Inventory - Inventory existing Handshake Atelier/Lens stubs
  - MT-010: Inventory - Inventory adjacent Photo/Studio/media/Loom/ASR/artifact stubs
  - MT-011: Inventory - Inventory prior packets for supersession risk
  - MT-012: Inventory - Inventory current Handshake product code anchors relevant to Atelier/Lens
  - MT-013: Requirement extraction - Extract media/DAM requirements
  - MT-014: Requirement extraction - Extract character sheet requirements
  - MT-015: Requirement extraction - Extract template/parser requirements
  - MT-016: Requirement extraction - Extract intake/inbox requirements
  - MT-017: Requirement extraction - Extract collection/contact-sheet requirements
  - MT-018: Requirement extraction - Extract sidecar/versioning requirements
  - MT-019: Requirement extraction - Extract PoseKit/OpenPose requirements
  - MT-020: Requirement extraction - Extract identity profile requirements
  - MT-021: Requirement extraction - Extract ComfyUI workflow requirements
  - MT-022: Requirement extraction - Extract automation/manual/debug requirements
  - MT-023: Requirement extraction - Extract search/tag/similarity requirements
  - MT-024: Requirement extraction - Extract export/backup/share-pack requirements
  - MT-025: Requirement extraction - Extract no-rewrite/no-censorship text preservation requirements
  - MT-026: Requirement extraction - Extract path/naming portability requirements
  - MT-027: Preservation - Preserve WP-1-Atelier-Lens-v2 gaps
  - MT-028: Preservation - Preserve WP-1-Photo-Studio-v2 gaps
  - MT-029: Preservation - Preserve WP-1-Atelier-Collaboration-Panel-v1 baseline
  - MT-030: Preservation - Preserve WP-1-Lens-Extraction-Tier-v1 scope
  - MT-031: Preservation - Preserve WP-1-Lens-ViewMode-v1 baseline
  - MT-032: Preservation - Preserve WP-1-Stage-Media-Artifact-Portability-v1 scope
  - MT-033: Preservation - Preserve WP-1-Stage-ASR-Transcript-Lineage-v1 scope
  - MT-034: Preservation - Preserve WP-1-Studio-Runtime-Visibility-v1 scope
  - MT-035: Preservation - Preserve Loom/media downloader/video archive adjacency
  - MT-036: Preservation - Preserve screenshot/visual-debug inherited requirements
  - MT-037: Preservation - Preserve artifact-system foundation dependencies
  - MT-038: Preservation - Preserve structured-collaboration/governance substrate dependencies
  - MT-039: Translation - Build SQLite absolute rejection row and no-test/no-fixture tripwire
  - MT-040: Translation - Build Electron rejection and Tauri translation row
  - MT-041: Translation - Build localhost intake rejection and Handshake endpoint translation row
  - MT-042: Translation - Build CKC namespace migration row
  - MT-043: Translation - Build product-output path hygiene row
  - MT-044: Translation - Build search architecture translation row
  - MT-045: Translation - Build automation lease/session translation row
  - MT-046: Translation - Build sidecar artifact translation row
  - MT-047: Translation - Build ComfyUI receipt translation row
  - MT-048: Translation - Build PoseKit schema translation row
  - MT-049: Translation - Build export/backup manifest translation row
  - MT-050: Translation - Build ViewMode and LensExtractionTier preservation row
  - MT-051: Translation - Build Atelier/Lens versus CKC overlap matrix
  - MT-052: Translation - Build CKC evolved-feature and convenience-driven requirement register
  - MT-053: Consolidation - Classify CKC extra features as folded, dependency, deferred, conflict, or operator-decision-needed
  - MT-054: Fixture selection - Select CKC sheet parser fixtures
  - MT-055: Fixture selection - Select protected field fixtures
  - MT-056: Fixture selection - Select character ID fixtures
  - MT-057: Fixture selection - Select media provenance fixtures
  - MT-058: Fixture selection - Select intake batch fixtures
  - MT-059: Fixture selection - Select OpenPose sidecar fixtures
  - MT-060: Fixture selection - Select PoseKit hand/body/face fixtures
  - MT-061: Fixture selection - Select ComfyUI bridge payload fixtures
  - MT-062: Fixture selection - Select automation manual/command-map fixtures
  - MT-063: Fixture selection - Select export/backup manifest fixtures
  - MT-064: Fixture selection - Select search/tag/similarity fixtures
  - MT-065: Proof definition - Define PostgreSQL-first proof expectations
  - MT-066: Packet authoring - Draft consolidation source requirement table
  - MT-067: Packet authoring - Draft consolidation out-of-scope guard
  - MT-068: Packet authoring - Draft consolidation acceptance gates
  - MT-069: Packet authoring - Draft deferred CKC Kernel source requirement notes
  - MT-070: Packet authoring - Draft deferred CKC Vertical Slice source requirement notes
  - MT-071: Packet authoring - Draft red-team section for consolidation
  - MT-072: Packet authoring - Draft red-team notes for deferred Kernel
  - MT-073: Packet authoring - Draft red-team notes for deferred Vertical Slice
  - MT-074: Runway closure - Validate all Greenroom JSON and source paths
  - MT-075: Runway closure - Produce corrected reference brief and handoff summary

## ACCEPTANCE_CRITERIA
  - Every source Atelier/Lens-adjacent stub is represented by a source-backed preservation row.
  - CKC is preserved as an evolved sibling of Atelier/Lens and prompt-diary intent, not treated as a competing app.
  - The overlap matrix maps CKC clusters to Handshake owners across Atelier/Lens, Photo Studio, Studio runtime, Loom/media/archive, artifact, and visual-debug surfaces.
  - Every CKC evolved or convenience feature is classified as fold, dependency, defer, conflict, or operator-decision-needed.
  - CKC runtime assumptions are translated to Handshake PostgreSQL/EventLedger/ArtifactStore/CRDT/promotion boundaries.
  - SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths. PostgreSQL/EventLedger/ArtifactStore are the only accepted storage and evidence direction.
  - Electron IPC, CKC localhost intake authority, .GOV product outputs, and CKC product namespace authority are rejected or translated.
  - Future CKC rebuild stubs are deferred until this packet, CKC greenroom review, and CKC research basis are complete.
  - The packet is detailed enough for no-context model execution and validator review without rereading all legacy stubs.
  - Packet, refinement, microtask, taskboard, traceability, inventory, and projection contracts validate with the packet truth bundle.

## VALIDATION_REQUIREMENTS
  - node .GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs --check
  - node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs
  - node -e "const fs=require('fs'); const c=fs.readdirSync('.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1').filter(f=>/^MT-\\d{3}\\.json$/.test(f)).length; if(c!==75) throw new Error(String(c));"
