---
file_id: ckc-atelier-lens-wp-stub-coverage-audit-20260516
file_kind: coverage_audit
updated_at: 2026-05-16
status: reference_only_not_execution_authority
wp_family: atelier_lens_ckc_fold_in
---

<topic id="audit-purpose" status="active" version="v1" summary="WP stub coverage repair audit" updated_at="2026-05-16">

# WP Stub Coverage Audit

This audit checks whether the three CKC fold-in WP stubs preserve the old Atelier/Lens-adjacent stubs and the CKC greenroom prep before any official microtask expansion.

Scope of this audit:

- `WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1`
- `WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1`
- `WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1`

The pre-MT coverage audit originally scoped microtask content out until the WP stub payload was repaired. A later draft-MT repair removed the poisoned 20-item inline drafts and replaced them with fresh non-executable draft suites; official MT files/contracts are still not generated.

</topic>

<topic id="coverage-verdict" status="active" version="v1" summary="Coverage verdict before repair" updated_at="2026-05-16">

# Coverage Verdict

Before this repair pass, the three stubs had the right high-level buckets but were not self-contained. They summarized or pointed at source maps instead of embedding the old-stub payload and CKC greenroom payload.

Repair target:

- Every old Atelier/Lens-adjacent stub is assigned to exactly one of the three new stubs as `folded`, `baseline_dependency`, `inherited_validation`, `separate_pattern_source`, or `historical_source`.
- Every CKC evolved feature `EVOL-001` through `EVOL-026` is explicitly assigned to an owner stub with preserved behavior and guardrails.
- Every overlap row `OVR-001` through `OVR-012` is explicitly assigned to an owner stub or cross-cutting boundary.
- Rejected runtime assumptions are visible in the stubs: SQLite, Electron authority, CKC namespace, localhost intake authority, product outputs under `.GOV`, and absolute machine-local paths.
- The stubs remain planning-only and non-executable.

</topic>

<topic id="repair-pass-status" status="active" version="v1" summary="Findings addressed in repaired stubs" updated_at="2026-05-16">

# Repair Pass Status

The findings and concerns from the pre-MT audit were addressed in the three CKC fold-in stubs before official MT generation.

Repaired surfaces:

- `WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1`: added prior-packet inspection requirements for sparse `WP-1-Atelier-Lens-v2` and `WP-1-Photo-Studio-v2`, active `WP-1-Media-Downloader-v2` payload, Media Downloader v1 historical details, ASR format/provisioning decisions, Loom video wrapper/caption/transcript details, full CKC docs/stories/moodboard/relationship/media-review/import/export/reset coverage, CKC taskboard evidence-maturity rules, and README/spec drift.
- `WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1`: added concrete Rig/OpenPose/IdentityProfile/WorkflowReceipt minimum fields, output-first registration failure recovery, external tool/model capability posture, workflow spec registry and image-sourcing adapter details, multi-rig state preservation, blocked `WP-0133` handling, and planned PoseKit carry-forward rows.
- `WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1`: added manual/action-catalog field requirements, session/lease/heartbeat detail, visual debugging loop details, local LLM/AI tagging visibility, build/release/install diagnostics, diagnostic bundle contents, stale-doc/spec drift detection, and CKC operational status-maturity rules.

Remaining gate:

- Official microtask generation remains blocked until activation/refinement approval and USER_SIGNATURE for the repaired three-stub family and draft MT suites.

</topic>

<topic id="draft-microtask-suite-repair" status="draft_non_executable" version="v1" summary="Poisoned 20-MT drafts replaced" updated_at="2026-05-16">

# Draft Microtask Suite Repair

The earlier inline 20-MT drafts were a self-imposed limit and are retired. They must not be reused as activation source material.

Fresh draft suites:

| Stub | Draft MT count | Draft suite path |
|---|---:|---|
| `WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1` | 80 | `.GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1/MT_SUITE.md` |
| `WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1` | 51 | `.GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1/MT_SUITE.md` |
| `WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1` | 66 | `.GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1/MT_SUITE.md` |

Execution status: non-executable planning only. Activation Manager must split any draft MT further if it touches unrelated files, crosses owner boundaries, or cannot be executed by a no-context local/small cloud model from that MT alone.

</topic>

<topic id="old-stub-owner-map" status="active" version="v1" summary="Old Handshake stub ownership mapping" updated_at="2026-05-16">

# Old Stub Owner Map

| Source stub | Owner in new stub family | Handling | Preserved payload |
|---|---|---|---|
| `WP-1-Atelier-Lens-v2` | Core Data/Intake | folded | Role claiming, SceneState, ConflictSet, role/provenance behavior, Lens proposal workflows, sparse-contract risk, Studio Runtime dependency. |
| `WP-1-Photo-Studio-v2` | Core Data/Intake | folded | Skeleton surface, thumbnails, recipes, media viewer/DAM responsibility, overlap with Loom preview thumbnails. |
| `WP-1-Atelier-Collaboration-Panel-v1` | Core Data/Intake | baseline_dependency | Selection-scoped role suggestions, multi-suggestion review, range-bounded patching, provenance, out-of-selection rejection, off-by-one risk. |
| `WP-1-Lens-Extraction-Tier-v1` | Core Data/Intake | folded | Tier0/Tier1/Tier2, Tier1 default, explicit Tier0/Tier2 override, requested/effective trace, invalid-tier rejection, content-tier confusion risk. |
| `WP-1-Lens-ViewMode-v1` | Core Data/Intake | baseline_dependency | NSFW default, explicit SFW toggle, SFW hard-drop projection, raw/derived immutability, trace-visible ViewMode filtering. |
| `WP-1-Stage-Media-Artifact-Portability-v1` | Core Data/Intake | folded | Shared artifact portability contract, manifests, bundle indexes, bounded export anchors, retention, Stage/media provenance. |
| `WP-1-Stage-ASR-Transcript-Lineage-v1` | Core Data/Intake | folded | Source media artifact to ASR input to transcript artifact to searchable consumer lineage, stable source hash, media probe facts, timing anchors. |
| `WP-1-Studio-Runtime-Visibility-v1` | Model Workflow/Diagnostics | folded_dependency | Runtime mappings for Studio surfaces, jobs, workflow nodes, tool surfaces, DCC, Flight Recorder, Locus/task-board/WP links, PostgreSQL-only state. |
| `WP-1-ASR-Transcribe-Media-v1` | Core Data/Intake | folded | Local-first ASR job, deterministic ffmpeg extraction, transcript JSON/text artifacts, timing metadata, caption skip path, cancellable/resumable posture. |
| `WP-1-Video-Archive-Loom-Integration-v1` | Core Data/Intake | folded | Video folder import to LoomBlocks, captions/transcripts as referenced entities, thumbnails/proxies, tags/mentions, Lens transcript retrieval, batch progress/resume. |
| `WP-1-Loom-MVP-v1` | Core Data/Intake | baseline_dependency | LoomBlocks, LoomEdges, stable UUID identity, mentions, tags, views, backlinks, file import, SHA-256 dedup, Tier-1 previews, search, FR-EVT-LOOM. |
| `WP-1-Loom-Storage-Portability-v4` | Core Data/Intake | baseline_dependency | PostgreSQL-only portability evidence, graph traversal, directional edges, metrics recomputation, source-anchor durability, registry-state inconsistency. |
| `WP-1-Loom-Preview-VideoPosterFrames-v1` | Core Data/Intake | folded | Deterministic poster-frame thumbnails, ThumbnailSpec resize, derived preview assets, LoomBlock links, ffmpeg/ffprobe capability gates, FR events. |
| `WP-1-Media-Downloader-v1` | Core Data/Intake | historical_source | YouTube/Instagram/forum/blog/generic media modes, captions VTT, full-res crawling, resumable queue, deterministic naming, sidecars, optional transcode, capability gates. |
| `WP-1-Media-Downloader-Loom-Bridge-v1` | Core Data/Intake | folded | Completed downloader artifact promotion to Asset/LoomBlock, content-hash dedup, captions/metadata sidecars, previews, searchable captions, FR events. |
| `WP-1-Product-Screenshot-Visual-Validation-v1` | Model Workflow/Diagnostics | inherited_validation | Programmatic full-window/panel/module capture, governed artifact storage, CLI/API trigger, screenshot metadata, Tauri/webview/native integration. |
| `WP-1-Visual-Debugging-Loop-v1` | Model Workflow/Diagnostics | inherited_validation | Generate-capture-compare-fix loop, visual diffs, validator evidence routing, threshold configuration, Tauri app test mode. |
| `WP-1-Calendar-Lens-v3` | Core Data/Intake | separate_pattern_source | Lens query/view pattern, date/source/title filters, stable IDs, provenance, optional read-only Flight Recorder overlay. |
| `WP-1-Artifact-System-Foundations-v1` | Core Data/Intake | foundational_dependency | Artifact store bootstrap, manifests, SHA-256, atomic Materialize API, retention/pinning/GC, no random filesystem side effects. |
| `WP-1-Structured-Collaboration-Artifact-Family-v1` | Model Workflow/Diagnostics | foundational_dependency | Structured packet/summary/index/thread artifacts, schema versioning, compact summaries, profile extensions, model-readable governance substrate. |

</topic>

<topic id="ckc-evolved-feature-owner-map" status="active" version="v1" summary="CKC evolved feature ownership mapping" updated_at="2026-05-16">

# CKC Evolved Feature Owner Map

| Feature | Owner stub | Handling |
|---|---|---|
| `EVOL-001` stable public character IDs | Core Data/Intake | folded |
| `EVOL-002` typed character sheet parser | Core Data/Intake | folded |
| `EVOL-003` append-only sheet versions | Core Data/Intake | folded |
| `EVOL-004` bulk character operations | Core Data/Intake | folded |
| `EVOL-005` persistent intake batches | Core Data/Intake | folded |
| `EVOL-006` contact sheets as artifacts | Core Data/Intake | folded |
| `EVOL-007` OpenPose sidecars hidden from normal galleries | Core Data/Intake plus Pose/ComfyUI | split: projection law in Core, pose sidecar production in Pose |
| `EVOL-008` PoseKit workbench contexts | Pose/ComfyUI | folded/deferred_family |
| `EVOL-009` Body-18/face-70/hand-21 rig contract | Pose/ComfyUI | folded |
| `EVOL-010` quaternion-backed yaw/pitch/roll head pose | Pose/ComfyUI | folded |
| `EVOL-011` identity profiles | Pose/ComfyUI | folded |
| `EVOL-012` multi-rig workspace tabs | Pose/ComfyUI | deferred_preserved |
| `EVOL-013` ComfyUI output registration and replay | Pose/ComfyUI | folded with runtime adaptation |
| `EVOL-014` workflow spec registry and image-sourcing adapter | Pose/ComfyUI | folded |
| `EVOL-015` identity-decoupled media filenames | Core Data/Intake plus Pose/ComfyUI | split: global artifact rule in Core, identity payload rule in Pose |
| `EVOL-016` global search with snippets and jump targets | Core Data/Intake | folded |
| `EVOL-017` tag manager, saved searches, palettes, dHash similarity | Core Data/Intake | folded |
| `EVOL-018` moodboard canvas inside character workflow | Core Data/Intake | folded |
| `EVOL-019` built-in model manual and command map | Model Workflow/Diagnostics | folded |
| `EVOL-020` sessions, leases, heartbeats, command logs | Model Workflow/Diagnostics | folded |
| `EVOL-021` non-focus-stealing automation and visual capture | Model Workflow/Diagnostics | folded |
| `EVOL-022` filesystem health and recoverable deletion | Core Data/Intake | folded |
| `EVOL-023` backup version traceability and orphan adoption | Core Data/Intake | folded |
| `EVOL-024` web portfolio and share pack exports | Core Data/Intake | folded |
| `EVOL-025` hybrid CRDT/event-log policy | Model Workflow/Diagnostics plus Core Data/Intake | split: global boundary in Diagnostics, data authority in Core |
| `EVOL-026` blocked PoseKit calibration/history debt | Pose/ComfyUI | deferred_preserved |

</topic>

<topic id="greenroom-overlap-owner-map" status="active" version="v1" summary="Greenroom overlap ownership mapping" updated_at="2026-05-16">

# Greenroom Overlap Owner Map

| Overlap row | Owner stub | Required handling |
|---|---|---|
| `OVR-001` character sheets and Atelier identity | Core Data/Intake | Preserve stable public/internal IDs, protected fields, append-only versions, selective merge/apply, byte-preserved user text, role/provenance. |
| `OVR-002` media viewer / DAM / Photo Studio | Core Data/Intake | Preserve browsing, thumbnails, metadata, provenance, missing-file diagnostics, archive/restore, sidecar hiding. |
| `OVR-003` intake / inbox / pending review | Core Data/Intake | Preserve persistent batches, accept/reject/pending, loose/linked modes, source preservation, resume. |
| `OVR-004` collections and contact sheets | Core Data/Intake | Preserve notes/tags, optional character/sheet-version links, source IDs/hashes, layout metadata, deferred raster export. |
| `OVR-005` sidecars, versioning, recovery | Core Data/Intake plus Pose/ComfyUI | Core owns sidecar projection/recovery/versioning; Pose owns pose sidecar production and schema. |
| `OVR-006` PoseKit / OpenPose / identity | Pose/ComfyUI | Preserve workbench modes, identity profile lineage, deterministic OpenPose sidecars, multi-rig tabs, calibration/history debt. |
| `OVR-007` ComfyUI workflow lineage | Pose/ComfyUI | Preserve receipts, prompt extraction, replay, stats, identity payloads, output registration, non-fatal failure; reject localhost authority. |
| `OVR-008` search, tags, links, similarity | Core Data/Intake | Preserve snippets, jump targets, tag manager, saved searches, backlinks, palettes, dHash similarity, AI tag suggestions; no SQLite FTS. |
| `OVR-009` docs, stories, moodboards, prompt diary intent | Core Data/Intake | Preserve docs inside character workflow, moodboard JSON, layers/folders, corkboard/outliner, links/backlinks, exports, text preservation. |
| `OVR-010` automation, model manual, visual diagnostics | Model Workflow/Diagnostics | Preserve command catalog, sessions/leases, heartbeats, command log, renderer state, captures, no OS-level input, no focus stealing, consistency tests. |
| `OVR-011` exports, backups, share packs, web portfolio | Core Data/Intake | Preserve no-space names, safe subsets, LLM packs, manifests, backup guards, orphan adoption, checksums, offline portfolio intent; no `.GOV` product outputs. |
| `OVR-012` parallel editing / event log / revisions | Model Workflow/Diagnostics plus Core Data/Intake | Preserve PostgreSQL authority, sessions/leases, EventLedger events, optimistic revisions, CRDT only for safe merge shapes. |

</topic>

<topic id="runtime-rejection-map" status="active" version="v1" summary="Rejected CKC runtime assumptions" updated_at="2026-05-16">

# Runtime Rejection Map

| Rejected item | Required Handshake replacement |
|---|---|
| SQLite in runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths | PostgreSQL/EventLedger authority and non-SQLite parity fixtures. |
| SQLite FTS5 search | Handshake search/index architecture using PostgreSQL-backed projections and retrieval contracts. |
| Electron main/preload as product authority | Tauri command facade plus Rust backend service/API boundaries. |
| CKC localhost intake authority | Typed Handshake integration/job receipt path with capability gates and EventLedger lineage. |
| CKC product namespace such as `CKC`, `CastKit Codex`, `CastKitCodexBridge` | Handshake/Atelier/Lens namespaces and portable no-space artifact names. |
| Product outputs under `.GOV` | Product runtime/artifact roots outside `.GOV`, with ArtifactStore manifests and retention. |
| Machine-local absolute source paths | Historical evidence only; runtime paths must be repo-relative, artifact-root-relative, or operator-configured. |
| Direct LLM execution of product operations | Governed AI Jobs, Workflow Engine nodes, MCP/tool policies, mechanical adapters, Flight Recorder/DCC evidence. |

</topic>
