<!-- HANDSHAKE_GENERATED_PROJECTION schema_id=hsk.work_packet_contract@1 source_file=.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/packet.json source_hash=07e45f94b7c9cf76 projection_hash=b916dd30064b9b91 generated_at_utc=2026-05-16T03:39:00.000Z generator=.GOV/roles_shared/scripts/wp/atelier-lens-consolidation-packet-generator.mjs -->
# WP-1-Atelier-Lens-Consolidation-v1: Atelier/Lens Consolidation and CKC Fold-In

## METADATA
- WP_ID: WP-1-Atelier-Lens-Consolidation-v1
- BASE_WP_ID: WP-1-Atelier-Lens-Consolidation
- **Status:** Ready for Dev
- DATE: 2026-05-16
- USER_SIGNATURE: ilja160520260339
- PACKET_FORMAT_VERSION: 2026-04-06
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
- RISK_TIER: HIGH
- CURRENT_WP_STATUS: READY_FOR_DEV
- MAIN_CONTAINMENT_STATUS: NOT_STARTED
- MERGED_MAIN_COMMIT: NONE
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
- WORKFLOW_AUTHORITY: ORCHESTRATOR
- TECHNICAL_ADVISOR: WP_VALIDATOR
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
- AGENTIC_MODE: NO
- EXECUTION_OWNER: CODER_A
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: HANDSHAKE_ACP_BROKER
- LOCAL_BRANCH: feat/WP-1-Atelier-Lens-Consolidation-v1
- LOCAL_WORKTREE_DIR: ../wtc-lens-consolidation-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Atelier-Lens-Consolidation-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Atelier-Lens-Consolidation-v1
- BACKUP_PUSH_STATUS: NOT_REQUIRED
- SESSION_HOST_FALLBACK: SYSTEM_TERMINAL_REPAIR_ONLY
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_ACP_DIRECT_HEADLESS_PRIMARY
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Atelier-Lens-Consolidation-v1
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Atelier-Lens-Consolidation-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-lens-consolidation-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Atelier-Lens-Consolidation-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Atelier-Lens-Consolidation-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Atelier-Lens-Consolidation-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Atelier-Lens-Consolidation-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Atelier-Lens-Consolidation-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Atelier-Lens-Consolidation-v1
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.5
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.4
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
- CODER_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- CODER_MODEL: gpt-5.5
- CODER_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- WP_VALIDATOR_MODEL: gpt-5.5
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_5_XHIGH
- INTEGRATION_VALIDATOR_MODEL: gpt-5.5
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Atelier-Lens-Consolidation-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Atelier-Lens-Consolidation-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Atelier-Lens-Consolidation-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Atelier-Lens-Consolidation-v1/RECEIPTS.jsonl
- WP_NOTIFICATIONS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Atelier-Lens-Consolidation-v1/NOTIFICATIONS.jsonl
- COMMUNICATION_CONTRACT: WP_COMMUNICATION_V1
- COMMUNICATION_HEALTH_GATE: REQUIRED_BEFORE_CLAIM
- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A
- PACKET_WIDENING_DECISION: NONE
- PACKET_WIDENING_EVIDENCE: N/A
- TOUCHED_FILE_BUDGET: 140
- BROAD_TOOL_ALLOWLIST: NONE
- DATA_CONTRACT_PROFILE: NONE
- REFINEMENT: .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.md
- MICROTASK_GLOB: .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/MT-*.md

## OPERATOR_REQUEST
The operator corrected the workflow: this is today's task. First consolidate all Atelier/Lens work packet stubs without losing their original intent, then fold CKC into that preserved Atelier/Lens runway, then only after greenroom and CKC research create CKC rebuild stubs. CKC is an evolved sibling of the same prompt-diary and Atelier/Lens goal, so its convenience features must be preserved and translated into Handshake instead of discarded.

## SCOPE_SUMMARY
This packet promotes the greenroom output into the official Ready for Dev consolidation packet. It is not product implementation. It turns CKC source evidence, CKC spec/taskboard evidence, and existing Handshake Atelier/Lens-adjacent stubs into a single execution contract, refinement, and 75 microtasks.

## IN_SCOPE_PATHS
  - .GOV/reference/ckc_atelier_lens_consolidation/**
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/**
  - .GOV/task_packets/stubs/WP-1-Atelier-Lens-Consolidation-v1.md
  - .GOV/task_packets/stubs/WP-1-Atelier-Lens-Consolidation-v1.contract.json
  - .GOV/roles_shared/records/TASK_BOARD.md
  - .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
  - .GOV/roles_shared/records/FLAT_PACKET_LEGACY_INVENTORY.json
  - .GOV/roles_shared/scripts/wp/atelier-lens-consolidation-packet-generator.mjs

## OUT_OF_SCOPE
  - src/**
  - app/**
  - tests/**
  - .GOV/spec/**
  - ../handshake_main/**
  - D:/Projects/LLM projects/CastKit-Codex/**

## SPEC_ANCHORS
  - .GOV/spec/SPEC_CURRENT.md
  - .GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#FEAT-ATELIER-LENS
  - .GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#FEAT-PHOTO-STUDIO
  - .GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#PRIM-Moodboard
  - .GOV/spec/master-spec-v02.185/spec-modules/12-end-of-file-appendices.md#TOOL-COMFYUI
  - .GOV/spec/master-spec-v02.185/spec-modules/10-product-surfaces.md#Photo-Studio-and-Library-DAM-functions
  - .GOV/operator/docs_local/handshake-v2-kernel-reset-brief.md#PostgreSQL-EventLedger-only-reset

## STORAGE_AND_RUNTIME_CONSTRAINTS
- SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths. PostgreSQL/EventLedger/ArtifactStore are the only accepted storage and evidence direction.
- CKC SQLite assumptions are rejected even for tests, fixtures, mocks, examples, fallback cache, temporary adapters, compatibility shims, imports, exports, or demo harnesses.
- CKC Electron IPC is source evidence only; Handshake implementation must use Handshake/Tauri command, window, event, and workspace contracts.
- CKC localhost intake authority is source evidence only; Handshake implementation must use governed ingestion endpoints, ArtifactStore receipts, EventLedger entries, and model-visible diagnostics.
- CKC product namespace and .GOV product-output habits are rejected; generated product data belongs in Handshake product surfaces, while repo governance stays under .GOV.

## SOURCE_EVIDENCE
- Greenroom output index: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-output-index.json
- Overlap matrix: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-overlap-matrix.json
- Evolved feature register: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-evolved-feature-register.json
- Requirements register: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-requirements-register.json
- Translation matrix: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-translation-matrix.json
- Microtask map: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-microtask-map.json
- Stub preservation map: .GOV/reference/ckc_atelier_lens_consolidation/handshake-stub-preservation-map.json

## PRESERVATION_REQUIREMENTS
  - WP-1-Atelier-Lens-v2: Additive remediation for failed or gapped Atelier Lens work.
  - WP-1-Photo-Studio-v2: Additive remediation for failed or gapped Photo Studio work.
  - WP-1-Atelier-Collaboration-Panel-v1: Implement a selection-scoped Atelier Collaboration Panel in editor surfaces.
  - WP-1-Lens-Extraction-Tier-v1: Implement LensExtractionTier as a first-class runtime and planning input.
  - WP-1-Lens-ViewMode-v1: Implement ViewMode UI and enforcement for Lens outputs.
  - WP-1-Stage-Media-Artifact-Portability-v1: Unify Stage capture/import sessions and Media Downloader outputs under portable artifact manifest, bundle-index, and retention semantics.
  - WP-1-Stage-ASR-Transcript-Lineage-v1: Define backend lineage from Stage-captured/imported media to governed ASR transcript artifacts.
  - WP-1-Studio-Runtime-Visibility-v1: Make Studio and Design Studio surfaces explicit runtime citizens.
  - WP-1-ASR-Transcribe-Media-v1: Implement local-first ASR transcription for audio/video media.
  - WP-1-Video-Archive-Loom-Integration-v1: Turn archived/imported video files into Loom library objects with searchable transcripts, captions sidecars, and tags/mentions that compose with Lens/Atelier.
  - WP-1-Loom-MVP-v1: Deliver the Phase 1 Loom MVP local-first library surface.
  - WP-1-Loom-Storage-Portability-v4: Re-open Loom portability as a narrow remediation/proof pass separating real portability evidence from narrative closure.
  - WP-1-Loom-Preview-VideoPosterFrames-v1: Support Tier-1 previews for video assets by generating deterministic poster-frame thumbnails as a background mechanical job.
  - WP-1-Media-Downloader-v1: Batch archive web media into local-first resumable ingest jobs with capability gating and evidence logging.
  - WP-1-Media-Downloader-Loom-Bridge-v1: Make Media Downloader outputs promotable into Loom as LoomBlocks.
  - WP-1-Product-Screenshot-Visual-Validation-v1: Build product-integrated screenshot capture for full app window, panels, and module-level views.
  - WP-1-Visual-Debugging-Loop-v1: Implement generate-capture-compare-fix visual debugging loop for GUI work packets.
  - WP-1-Calendar-Lens-v3: Implement Calendar Lens as first-class UI and API workflow.
  - WP-1-Artifact-System-Foundations-v1: Ensure artifact system foundations across exports and jobs.
  - WP-1-Structured-Collaboration-Artifact-Family-v1: Define canonical structured collaboration artifacts for work packets, microtasks, task board projections, and role mailbox exports.

## OVERLAP_MATRIX_SUMMARY
- OVR-001: Character sheets and Atelier identity -> fold_into_atelier_lens_core; preserve: stable public/internal IDs,protected fields,append-only sheet versions,selective merge/apply,byte-preserved user text,role/provenance rules
- OVR-002: Media viewer / DAM / Photo Studio -> fold_with_artifact_store_dependency; preserve: image-first browsing,thumbnails,metadata,provenance,missing-file diagnostics,archive/restore,sidecar hiding
- OVR-003: Intake / Inbox / pending review -> fold_as_atelier_intake_subsystem; preserve: persistent batches,accept/reject/pending,loose/linked modes,source preservation,character/sheet/collection linkage,resume after route switch/restart
- OVR-004: Collections and contact sheets -> fold_with_raster_export_deferred; preserve: notes/tags,optional character/sheet-version links,SVG/contact-sheet manifests,source IDs/hashes,layout metadata,planned PNG/JPG path
- OVR-005: Sidecars, versioning, recovery -> fold_with_artifact_and_event_lineage; preserve: sidecar visibility projection,archive/restore,optimistic revision checks,append-only events,deletion preview,no silent source deletion
- OVR-006: PoseKit / OpenPose / identity -> fold_as_deferred_ckc_atelier_feature_family; preserve: blank/single/collection workbench modes,identity profile lineage,deterministic OpenPose sidecars,multi-rig tabs,blocked calibration/history debt
- OVR-007: ComfyUI workflow lineage -> fold_intent_reject_localhost_authority; preserve: workflow receipts,prompt extraction,replay,stats,identity reference payloads,output image registration,non-fatal bridge failure posture
- OVR-008: Search, tags, links, similarity -> fold_into_lens_projection_search_layer; preserve: snippets,jump targets,tag manager,saved searches,backlinks,palettes,dHash similarity,AI tag suggestions
- OVR-009: Docs, stories, moodboards, prompt diary intent -> fold_into_atelier_lens_creative_planning_layer; preserve: docs inside character workflow,moodboard structured JSON,layers/folders,corkboard/outliner,links/backlinks,exports,text preservation
- OVR-010: Automation, model manual, visual diagnostics -> fold_as_model_operation_requirement; preserve: command catalog,sessions/leases,heartbeats,command log,renderer state,captures,no OS-level input,no focus stealing,manual/command consistency tests
- OVR-011: Exports, backups, share packs, web portfolio -> fold_through_artifact_export_job_layer; preserve: no-space names,safe subsets,LLM packs,manifests,backup version guard,orphan adoption,checksums,offline portfolio intent
- OVR-012: Parallel editing / event log / revisions -> fold_into_eventledger_crdt_boundary; preserve: PostgreSQL source of truth,sessions/leases,EventLedger events,optimistic revisions,CRDT only for safe merge shapes

## CKC_EVOLVED_FEATURES
- EVOL-001: Stable public character IDs separate from internal IDs -> fold; why: Operator-facing character identity needs stable labels without leaking storage keys.
- EVOL-002: Typed character sheet parser with union and block-list fields -> fold; why: CKC solved real sheet editing pain with typed fields, descriptor fallbacks, score normalization, and nested block editing.
- EVOL-003: Append-only sheet versions with selective apply/revert -> fold; why: Prevents data loss when models or imports update sheets.
- EVOL-004: Bulk character operations -> fold; why: Daily library work requires multi-select, bulk tags/fields, batch exports, trash, and restore.
- EVOL-005: Persistent Intake batches -> fold; why: CKC moved beyond one-off import into recoverable review work.
- EVOL-006: Contact sheets as artifacts -> fold; why: Contact sheets are useful for comparing generated or curated media and for handoff.
- EVOL-007: OpenPose sidecars hidden from normal galleries -> fold; why: Sidecars are production artifacts but clutter normal viewing unless projected intentionally.
- EVOL-008: PoseKit blank/single/collection workbench contexts -> fold_deferred_feature_family; why: Operator needs PoseKit to operate on collections and individual photos, not only a single character page.
- EVOL-009: Body-18 / face-70 / hand-21 rig contract -> fold_deferred_feature_family; why: CKC already hardened an OpenPose-compatible pose representation useful for ComfyUI and image production.
- EVOL-010: Quaternion-backed yaw/pitch/roll head pose -> fold; why: Fine control over generated character pose was an evolved production need.
- EVOL-011: Identity profiles for face-reference workflows -> fold; why: Stable identity references are directly tied to character consistency and ComfyUI workflows.
- EVOL-012: Multi-rig workspace tabs -> defer_after_kernel; why: Real pose workflow needs multiple drafts open without deleting stored rigs.
- EVOL-013: ComfyUI output registration and replay -> fold_intent_adapt_runtime; why: CKC already closed the loop from generation output back into character/media lineage.
- EVOL-014: Workflow spec registry and image-sourcing adapter -> fold; why: External generation/sourcing task output needs versioned ingestion and idempotency.
- EVOL-015: Identity-decoupled media filenames -> fold; why: Prevents character names or sensitive sheet fields from leaking into paths or events.
- EVOL-016: Global search with snippets and jump targets -> fold; why: Fast retrieval across sheets, notes, images, and moodboards is core Lens behavior.
- EVOL-017: Tag manager, saved searches, palettes, dHash similarity -> fold; why: Convenience features became library-scale navigation requirements.
- EVOL-018: Moodboard canvas inside character workflow -> fold; why: Prompt diaries and visual planning need more than static notes.
- EVOL-019: Built-in model manual and command map -> fold; why: Lets no-context models operate the product.
- EVOL-020: Sessions, leases, heartbeats, command logs -> fold_with_stronger_durability; why: CKC evolved toward multi-agent operation before Handshake's kernel reset.
- EVOL-021: Non-focus-stealing automation and visual capture -> fold; why: Model-driven GUI verification must be quiet and reproducible.
- EVOL-022: Filesystem health and recoverable deletion -> fold; why: Real media libraries drift; deletion must be reversible and explainable.
- EVOL-023: Backup version traceability and orphan adoption -> fold; why: Recovery became a first-class operator need.
- EVOL-024: Web portfolio and share pack exports -> fold; why: Operator needs portable handoff outputs.
- EVOL-025: Hybrid CRDT/event-log policy -> fold_into_kernel_boundary; why: CKC discovered the right boundary: not everything needs CRDT, but parallel edits need receipts and revisions.
- EVOL-026: Blocked PoseKit calibration/history debt -> defer_and_preserve; why: Unfinished work is still requirement evidence: draggable calibration, missing-marker placement, 3D/live split editing, forked history.

## TRANSLATION_REQUIREMENTS
  - storage: SQLite, FTS5, runtime DDL, SQL translator, db/codex.db fixtures => PostgreSQL/EventLedger authority only. SQLite is not accepted in Handshake in any form, including runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths.
  - app_shell: Electron main/preload, BrowserWindow, IPC bridge => Tauri/Rust command boundary and React projection
  - intake: Localhost CKC intake endpoint as runtime authority => Typed Handshake endpoint or artifact proposal path with EventLedger lineage
  - namespace: CKC/CastKit product names and paths => Handshake/Atelier/Lens namespace and portable no-space artifact names
  - product_artifacts: CKC outputs under CKC_GOV/targets => Product runtime/artifact roots, not .GOV
  - search: SQLite FTS5 implementation details => Preserve behavior and use Handshake search/index architecture
  - automation: Process-local sessions/leases => Postgres/EventLedger-backed leases and receipts
  - ui_scope: CKC feature-rich React/Electron views => Later React projections after kernel contracts; no GUI parity in Kernel WP

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

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Source-stub no-loss preservation | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/handshake-stub-preservation-map.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: preservation stubs and carried-forward intent rows | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: CKC/Atelier overlap matrix | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-overlap-matrix.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: OVR-001 through OVR-012 | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: CKC evolved features | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-evolved-feature-register.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: EVOL-001 through EVOL-026 | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Runtime translation matrix | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-translation-matrix.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: module boundaries and conflict rows | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: SQLite absolute rejection | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-output-index.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/packet.md | TESTS: rg -n "SQLite" .GOV/reference/ckc_atelier_lens_consolidation .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1 | EXAMPLES: no runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Future CKC rebuild stubs gated | CODE_SURFACES: .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/packet.md, .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: deferred downstream WP register | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Model-facing manual diagnostics non-focus automation | CODE_SURFACES: .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.md, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/MT-*.md | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: manual, visual debug, structured receipts, quiet model operation | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: 75 MT coverage | CODE_SURFACES: .GOV/reference/ckc_atelier_lens_consolidation/greenroom-microtask-map.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/MT-*.json | TESTS: node -e "const fs=require('fs'); const c=fs.readdirSync('.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1').filter(f=>/^MT-\\d{3}\\.json$/.test(f)).length; if(c!==75) throw new Error(String(c));" | EXAMPLES: MT-001 through MT-075 | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Task Board and registry point at official packet | CODE_SURFACES: .GOV/roles_shared/records/TASK_BOARD.md, .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md | TESTS: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | EXAMPLES: Ready for Dev entry and active registry row | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Contracts and projections stay in sync | CODE_SURFACES: .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/*.json, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/*.md | TESTS: node .GOV/roles_shared/checks/packet-contract-projection-check.mjs | EXAMPLES: generated projection headers and source hashes | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING

## PACKET_ACCEPTANCE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the executable acceptance contract for packet closure. New packets must keep stable row IDs and move each required row to PROVED, CONFIRMED, or NOT_APPLICABLE with evidence before PASS.
- Rule: use STEER or BLOCKED for unresolved required rows instead of narrative closure.
- ACCEPTANCE_ROWS:
  - ID: AC-001 | REQUIREMENT: Source-stub no-loss preservation | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | REASON: NONE
  - ID: AC-002 | REQUIREMENT: CKC/Atelier overlap matrix | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | REASON: NONE
  - ID: AC-003 | REQUIREMENT: CKC evolved features | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | REASON: NONE
  - ID: AC-004 | REQUIREMENT: Runtime translation matrix | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | REASON: NONE
  - ID: AC-005 | REQUIREMENT: SQLite absolute rejection | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: rg -n "SQLite" .GOV/reference/ckc_atelier_lens_consolidation .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1 | REASON: NONE
  - ID: AC-006 | REQUIREMENT: Future CKC rebuild stubs gated | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | REASON: NONE
  - ID: AC-007 | REQUIREMENT: Model-facing manual diagnostics non-focus automation | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | REASON: NONE
  - ID: AC-008 | REQUIREMENT: 75 MT coverage | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: node -e "const fs=require('fs'); const c=fs.readdirSync('.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1').filter(f=>/^MT-\\d{3}\\.json$/.test(f)).length; if(c!==75) throw new Error(String(c));" | REASON: NONE
  - ID: AC-009 | REQUIREMENT: Task Board and registry point at official packet | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs | REASON: NONE
  - ID: AC-010 | REQUIREMENT: Contracts and projections stay in sync | REQUIRED: YES | EVIDENCE_KIND: CLAUSE_CLOSURE_MATRIX | OWNER: WP_VALIDATOR | STATUS: PENDING | EVIDENCE: node .GOV/roles_shared/checks/packet-contract-projection-check.mjs | REASON: NONE

## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.

## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/packet.md
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/packet.json
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.md
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/MT-*.md
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/MT-*.json
  - .GOV/roles_shared/records/TASK_BOARD.md
  - .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
  - .GOV/roles_shared/records/FLAT_PACKET_LEGACY_INVENTORY.json
  - .GOV/reference/ckc_atelier_lens_consolidation/**
- REQUIRED_TRIPWIRE_TESTS:
  - node .GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs --check
  - node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs
  - node -e "const fs=require('fs'); const c=fs.readdirSync('.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1').filter(f=>/^MT-\\d{3}\\.json$/.test(f)).length; if(c!==75) throw new Error(String(c));"
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.


## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: WAIVED_NOT_DATA_BEARING
- REASON: This activation packet writes governance packet/refinement/microtask surfaces only. It does not implement product data schemas, migrations, runtime persistence, or product code.
- EVIDENCE:
  - IN_SCOPE_PATHS reviewed: .GOV/reference/ckc_atelier_lens_consolidation/**, .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/**, .GOV/task_packets/stubs/WP-1-Atelier-Lens-Consolidation-v1.md, .GOV/task_packets/stubs/WP-1-Atelier-Lens-Consolidation-v1.contract.json, .GOV/roles_shared/records/TASK_BOARD.md, .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md, .GOV/roles_shared/records/FLAT_PACKET_LEGACY_INVENTORY.json, .GOV/roles_shared/scripts/wp/atelier-lens-consolidation-packet-generator.mjs
  - No src, app, tests, migration, backend storage, schema, DTO, or product runtime path is in scope.
  - SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths. PostgreSQL/EventLedger/ArtifactStore are the only accepted storage and evidence direction.



## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: NO
- SQL_POSTURE: NOT_APPLICABLE
- LLM_READABILITY_POSTURE: NOT_APPLICABLE
- LOOM_INTERTWINED_POSTURE: NOT_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - NONE
- DATA_CONTRACT_RULES:
  - NONE
- VALIDATOR_DATA_PROOF_HINTS:
  - NONE


## MICROTASK_PLAN
  - MT-001: Inventory CKC package/runtime dependencies
  - MT-002: Inventory CKC backend service files
  - MT-003: Inventory CKC UI views and reusable behavior
  - MT-004: Inventory CKC PoseKit files
  - MT-005: Inventory CKC ComfyUI bridge files
  - MT-006: Inventory CKC test suite by behavior area
  - MT-007: Inventory CKC spec headings and requirement sections
  - MT-008: Inventory CKC taskboard statuses
  - MT-009: Inventory existing Handshake Atelier/Lens stubs
  - MT-010: Inventory adjacent Photo/Studio/media/Loom/ASR/artifact stubs
  - MT-011: Inventory prior packets for supersession risk
  - MT-012: Inventory current Handshake product code anchors relevant to Atelier/Lens
  - MT-013: Extract media/DAM requirements
  - MT-014: Extract character sheet requirements
  - MT-015: Extract template/parser requirements
  - MT-016: Extract intake/inbox requirements
  - MT-017: Extract collection/contact-sheet requirements
  - MT-018: Extract sidecar/versioning requirements
  - MT-019: Extract PoseKit/OpenPose requirements
  - MT-020: Extract identity profile requirements
  - MT-021: Extract ComfyUI workflow requirements
  - MT-022: Extract automation/manual/debug requirements
  - MT-023: Extract search/tag/similarity requirements
  - MT-024: Extract export/backup/share-pack requirements
  - MT-025: Extract no-rewrite/no-censorship text preservation requirements
  - MT-026: Extract path/naming portability requirements
  - MT-027: Preserve WP-1-Atelier-Lens-v2 gaps
  - MT-028: Preserve WP-1-Photo-Studio-v2 gaps
  - MT-029: Preserve WP-1-Atelier-Collaboration-Panel-v1 baseline
  - MT-030: Preserve WP-1-Lens-Extraction-Tier-v1 scope
  - MT-031: Preserve WP-1-Lens-ViewMode-v1 baseline
  - MT-032: Preserve WP-1-Stage-Media-Artifact-Portability-v1 scope
  - MT-033: Preserve WP-1-Stage-ASR-Transcript-Lineage-v1 scope
  - MT-034: Preserve WP-1-Studio-Runtime-Visibility-v1 scope
  - MT-035: Preserve Loom/media downloader/video archive adjacency
  - MT-036: Preserve screenshot/visual-debug inherited requirements
  - MT-037: Preserve artifact-system foundation dependencies
  - MT-038: Preserve structured-collaboration/governance substrate dependencies
  - MT-039: Build SQLite absolute rejection row and no-test/no-fixture tripwire
  - MT-040: Build Electron rejection and Tauri translation row
  - MT-041: Build localhost intake rejection and Handshake endpoint translation row
  - MT-042: Build CKC namespace migration row
  - MT-043: Build product-output path hygiene row
  - MT-044: Build search architecture translation row
  - MT-045: Build automation lease/session translation row
  - MT-046: Build sidecar artifact translation row
  - MT-047: Build ComfyUI receipt translation row
  - MT-048: Build PoseKit schema translation row
  - MT-049: Build export/backup manifest translation row
  - MT-050: Build ViewMode and LensExtractionTier preservation row
  - MT-051: Build Atelier/Lens versus CKC overlap matrix
  - MT-052: Build CKC evolved-feature and convenience-driven requirement register
  - MT-053: Classify CKC extra features as folded, dependency, deferred, conflict, or operator-decision-needed
  - MT-054: Select CKC sheet parser fixtures
  - MT-055: Select protected field fixtures
  - MT-056: Select character ID fixtures
  - MT-057: Select media provenance fixtures
  - MT-058: Select intake batch fixtures
  - MT-059: Select OpenPose sidecar fixtures
  - MT-060: Select PoseKit hand/body/face fixtures
  - MT-061: Select ComfyUI bridge payload fixtures
  - MT-062: Select automation manual/command-map fixtures
  - MT-063: Select export/backup manifest fixtures
  - MT-064: Select search/tag/similarity fixtures
  - MT-065: Define PostgreSQL-first proof expectations
  - MT-066: Draft consolidation source requirement table
  - MT-067: Draft consolidation out-of-scope guard
  - MT-068: Draft consolidation acceptance gates
  - MT-069: Draft deferred CKC Kernel source requirement notes
  - MT-070: Draft deferred CKC Vertical Slice source requirement notes
  - MT-071: Draft red-team section for consolidation
  - MT-072: Draft red-team notes for deferred Kernel
  - MT-073: Draft red-team notes for deferred Vertical Slice
  - MT-074: Validate all Greenroom JSON and source paths
  - MT-075: Produce corrected reference brief and handoff summary

## VALIDATION_PLAN
  - Parse all generated JSON contracts and reference registers.
  - Run node .GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs --check.
  - Run node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs.
  - Confirm 75 MT JSON contracts exist and all generated projection headers are in sync.
  - Search new packet/reference surfaces for weak SQLite language and stale Greenroom-WP phrasing.

## VALIDATION_REPORTS
- Status: PENDING
- CLAUSES_REVIEWED:
  - NONE
- NOT_PROVEN:
  - All clause rows pending coder execution and validator confirmation.
- SPEC_ALIGNMENT_VERDICT: PENDING

## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - node .GOV/roles_shared/scripts/wp/task-packet-stub-contracts.mjs --check
  - node .GOV/roles_shared/checks/packet-truth-bundle-check.mjs
  - node -e "const fs=require('fs'); const c=fs.readdirSync('.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1').filter(f=>/^MT-\\d{3}\\.json$/.test(f)).length; if(c!==75) throw new Error(String(c));"
- CANONICAL_CONTRACT_EXAMPLES:
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/packet.json (work_packet_contract schema)
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.json (refinement_contract schema)
  - .GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/MT-001.json through MT-075.json (microtask_contract schema)
  - .GOV/reference/ckc_atelier_lens_consolidation/greenroom-overlap-matrix.json (CKC/Atelier overlap rows OVR-001..OVR-012)
  - .GOV/reference/ckc_atelier_lens_consolidation/greenroom-evolved-feature-register.json (CKC evolved features EVOL-001..EVOL-026)
  - .GOV/reference/ckc_atelier_lens_consolidation/handshake-stub-preservation-map.json (Atelier/Lens stub preservation rows)
  - .GOV/reference/ckc_atelier_lens_consolidation/greenroom-translation-matrix.json (Handshake runtime translation rows)
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: PREPARE assignment pending; WP communications will record live progress.
