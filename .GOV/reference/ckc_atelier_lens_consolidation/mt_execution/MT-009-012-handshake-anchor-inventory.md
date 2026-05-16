# MT-009-012 Handshake Anchor Inventory

## Status

- WP: `WP-1-Atelier-Lens-Consolidation-v1`
- MTs covered: `MT-009`, `MT-010`, `MT-011`, `MT-012`
- Status: complete read-only inventory
- Write scope used:
  - `.GOV/reference/ckc_atelier_lens_consolidation/mt_execution/MT-009-012-handshake-anchor-inventory.md`
  - `.GOV/reference/ckc_atelier_lens_consolidation/mt_execution/MT-009-012-handshake-anchor-inventory.json`

## Hard Constraints

SQLite is absolutely rejected for Handshake: no runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths may use SQLite. PostgreSQL/EventLedger/ArtifactStore are the only accepted storage and evidence direction for this consolidation.

CKC is source evidence only. This inventory creates no CKC rebuild stubs and does not grant CKC runtime, namespace, Electron, localhost-intake, or SQLite authority in Handshake.

This worktree is governance-kernel-only. `GOV_KERNEL_ONLY.md` states that product code (`src/`, `app/`, `tests/`) does not belong here, so product anchors are candidates from packet evidence, not locally verified source files.

## Sources Read

- `GOV_KERNEL_ONLY.md`
- `.GOV/reference/ckc_atelier_lens_consolidation/handshake-stub-preservation-map.json`
- `.GOV/reference/ckc_atelier_lens_consolidation/consolidated-runway.json`
- `.GOV/reference/ckc_atelier_lens_consolidation/greenroom-overlap-matrix.json`
- `.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/packet.json`
- `.GOV/task_packets/WP-1-Atelier-Lens-Consolidation-v1/refinement.md`
- `.GOV/roles_shared/records/TASK_BOARD.md`
- `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
- Targeted packet/stub evidence under `.GOV/task_packets` and `.GOV/task_packets/stubs`

`rg` was used first for broad and narrowed discovery across stubs, task packets, task board, traceability registry, specs, and non-`.GOV` product-surface checks.

## MT-009 Existing Atelier/Lens Stubs

| WP | State | Preserved intent | Dependency / overlap notes |
| --- | --- | --- | --- |
| `WP-1-Atelier-Lens-v2` | Stub backlog; supersedes base `WP-1-Atelier-Lens` | Preserve role claiming, `SceneState`, and `ConflictSet` gaps. | Fold into Atelier/Lens core; sparse machine contract means Markdown projection is important source evidence. |
| `WP-1-Atelier-Collaboration-Panel-v1` | Validated baseline | Selection-scoped role suggestions, range-bounded apply, out-of-selection rejection, provenance, evidence refs. | Reuse for CKC/Atelier sheet and text-edit workflows; do not reopen as a new stub. |
| `WP-1-Lens-Extraction-Tier-v1` | Stub backlog | Tier0/Tier1/Tier2, Tier1 default, explicit override, requested/effective trace, invalid-tier validation. | Feeds Lens projection/search behavior; avoid confusion with content tier and implicit Tier2 creep. |
| `WP-1-Lens-ViewMode-v1` | Validated baseline | NSFW default, explicit SFW toggle, SFW hard-drop projection, immutable raw/derived artifacts, trace-visible filter. | Reuse projection law; ViewMode must not mutate raw evidence. |
| `WP-1-Calendar-Lens-v3` | Adjacent stub backlog | Calendar Lens UI/API workflow, filters, title search, stable IDs, provenance, optional timeline/FR overlay. | Lens-pattern source only; keep separate from media/Atelier consolidation unless operator expands scope. |

Primary source paths: `.GOV/task_packets/stubs/*Atelier*`, `.GOV/task_packets/stubs/*Lens*`, active packet/refinement, task board, and traceability registry.

## MT-010 Adjacent Photo/Studio/Media/Loom/ASR/Artifact Stubs

| WP | State | Preserved intent | Dependency / overlap notes |
| --- | --- | --- | --- |
| `WP-1-Photo-Studio-v2` | Stub backlog; supersedes base `WP-1-Photo-Studio` | Skeleton surface, thumbnails, recipes. | Overlaps CKC media viewer/DAM, character media panes, thumbnails, and ComfyUI recipe lineage. |
| `WP-1-Studio-Runtime-Visibility-v1` | Stub backlog | Studio/Design Studio runtime citizenship through jobs, workflow nodes, tools, DCC/operator projection, FR, Locus, and PostgreSQL-only state. | Bridge for CKC automation/manual/debug and ComfyUI lineage. |
| `WP-1-Stage-Media-Artifact-Portability-v1` | Stub backlog; blocked/high risk | Portable artifact manifests, bundle indexes, retention semantics, bounded exports, storage-portable evidence. | Foundation should precede downstream Loom bridge/archive packets. |
| `WP-1-Stage-ASR-Transcript-Lineage-v1` | Stub backlog; blocked/high risk | Source media to ASR input to transcript artifact to searchable consumer lineage with hashes, probe facts, provenance, and timing anchors. | Shared identity contract for Stage, ASR, Loom, Lens, and archive. |
| `WP-1-ASR-Transcribe-Media-v1` | Stub backlog | Local-first ASR job, deterministic ffmpeg extraction, transcript artifacts, Loom/search attachment, capability gating, FR progress. | Supports video archive search and transcript Lens retrieval. |
| `WP-1-Video-Archive-Loom-Integration-v1` | Stub backlog; operator-requested | Archived/imported videos as Loom objects with transcripts, captions sidecars, tags/mentions, thumbnails/proxies, Lens/Atelier composition. | Depends on Media Downloader, Loom MVP, Loom bridge, poster frames, and ASR. |
| `WP-1-Media-Downloader-Loom-Bridge-v1` | Stub backlog | Promote downloader outputs into LoomBlocks with captions, metadata sidecars, previews, search, and FR events. | Consumes active Media Downloader v2 and Loom MVP; avoid artifact/asset duplication. |
| `WP-1-Loom-Preview-VideoPosterFrames-v1` | Stub backlog | Deterministic Tier-1 poster-frame thumbnails for video assets. | Needs ffmpeg/ffprobe capability gates and deterministic error handling. |
| `WP-1-Loom-MVP-v1` | Validated baseline | LoomBlocks, LoomEdges, stable UUIDs, tags/mentions, views, backlinks, import/dedup, previews, search, FR-EVT-LOOM. | Reuse as baseline; historical SQLite references are superseded by the no-SQLite rule. |
| `WP-1-Loom-Storage-Portability-v4` | Validated baseline with mixed registry wording | Narrow portability/proof pass for graph traversal, directional edges, metrics recomputation, and source-anchor durability. | Use v4 as proof baseline; reconcile task-board/registry wording later. |
| `WP-1-Artifact-System-Foundations-v1` | Validated foundational dependency | ArtifactStore bootstrap, manifests, SHA-256, Materialize API, retention/pinning/GC, no random filesystem side effects. | Required for CKC media, ComfyUI receipts, exports, backups, contact sheets, sidecars, and validation evidence. |

## MT-011 Supersession Risks

- `WP-1-Atelier-Lens` and `WP-1-Atelier-Lens-v0.1`: historical/superseded evidence. Risk: earlier intent gets overwritten by CKC scope. Mitigation: preserve v2 gaps explicitly.
- `WP-1-Photo-Studio` and `WP-1-Photo-Studio-Skeleton`: historical/superseded evidence. Risk: skeleton/thumbnails/recipes get lost under CKC media scope. Mitigation: keep v2 intent explicit.
- `WP-1-Media-Downloader-v1`: superseded by validated `WP-1-Media-Downloader-v2`. Risk: v1 source-mode details are lost. Mitigation: use v2 as active dependency and preserve v1 as historical source material.
- `WP-1-Loom-Storage-Portability-v1/v2/v3`: superseded or failed-historical lineage; v4 is current validated recovery. Risk: stale closure claims contaminate CKC/Loom planning. Mitigation: use v4 only as current proof baseline.
- `WP-1-Product-Screenshot-Visual-Validation-v1` and `WP-1-Visual-Debugging-Loop-v1`: superseded and folded into Kernel002. Risk: visual evidence requirements disappear. Mitigation: inherit via Kernel visual surfaces.
- Archived premature CKC stubs under `.GOV/task_packets/_archive/superseded/premature-ckc-stubs-20260516/`: evidence only. Risk: bypassing consolidation and research. Mitigation: do not revive directly or create CKC rebuild stubs now.

## MT-012 Product Code Anchor Candidates

Local status: no product code is present in this worktree. Candidate anchors from packet evidence:

| Anchor | Candidate paths | Evidence source | Relevance |
| --- | --- | --- | --- |
| Atelier collaboration UI | `app/src/components/AtelierCollaborationPanel.tsx`, `DocumentView.tsx`, `TiptapEditor.tsx`, `app/src/lib/api.ts` | `WP-1-Atelier-Collaboration-Panel-v1/packet.md` | Selection-scoped Atelier UI/API surface. |
| Atelier collaboration backend | `src/backend/handshake_core/src/ace/validators/atelier_scope.rs`, `api/workspaces.rs`, `workflows.rs`, tests | `WP-1-Atelier-Collaboration-Panel-v1/packet.md` | Selection-bounded patching, provenance, validator enforcement. |
| Loom backend | `src/backend/handshake_core/src/api/loom.rs`, `storage/loom.rs`, `storage/postgres.rs`, `loom_fs.rs`, `workflows.rs` | `WP-1-Loom-MVP-v1/packet.md` | LoomBlock/LoomEdge identity, import/dedup, preview jobs, search, FR events. |
| Media downloader UI/Tauri | `app/src/components/MediaDownloaderView.tsx`, `app/src/lib/mediaDownloader.ts`, `app/src-tauri/src/lib.rs` | `WP-1-Media-Downloader-v2/packet.md` | Media archive worksurface, output-root posture, Stage Sessions/cookie handling. |
| Media downloader workflows | `src/backend/handshake_core/src/workflows.rs`, `api/jobs.rs`, `flight_recorder/mod.rs` | `WP-1-Media-Downloader-v2/packet.md` | Job family, progress events, process handling, capability posture. |
| Artifact foundation | Product ArtifactStore/EventLedger/Materialize surfaces to verify in product worktree | `WP-1-Artifact-System-Foundations-v1/packet.md` | Required for receipts, manifests, retention, contact sheets, exports, and validation evidence. |
| Kernel boundaries | KB001 EventLedger/SessionBroker, KB002 CRDT Write Box, KB003 Sandbox/Validation/Promotion | Consolidation refinement | Required for parallel editing, revisions, sessions, leases, and promotion evidence. |

Anchor caveat: every product path above must be re-verified in the product worktree before implementation. Any old packet evidence that mentions SQLite, FTS5, SQLite fixtures, or SQLite tests is historical only and must be translated to PostgreSQL/EventLedger/ArtifactStore or rejected.

## Key Findings

- Existing Atelier/Lens intent is split between active consolidation, stub backlog items, validated baselines, and adjacent Lens-pattern stubs.
- Adjacent Photo/Studio/media/Loom/ASR/artifact work supplies most of the reusable implementation runway, but several items are blocked or only source stubs.
- Supersession risk is real around old Atelier/Lens, Photo Studio, Media Downloader v1, Loom portability v1-v3, superseded visual-debug stubs, and archived premature CKC rebuild stubs.
- The governance-kernel worktree cannot verify product code directly; candidate anchors must be checked in the product worktree before implementation.
