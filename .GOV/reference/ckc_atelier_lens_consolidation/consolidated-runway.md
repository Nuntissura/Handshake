---
file_id: ckc-atelier-lens-consolidated-runway
file_kind: reference_runway
updated_at: 2026-05-16
status: reference_only_not_execution_authority
---

# CKC Atelier/Lens Consolidated Runway

This reference combines:
- `ckc-code-greenroom.md`
- `ckc-spec-taskboard-map.md`
- `handshake-stub-preservation-map.md`

It is not an executable work packet. It is source material for later refinement, signature, official packet creation, and task-board/build-order sync.

## Consolidation Law

- Preserve CKC intent before changing implementation shape.
- Preserve all existing Atelier/Lens-adjacent stub intent before consolidating.
- Treat CKC as an evolved sibling expression of Atelier/Lens and prompt-diary intent, not as optional inspiration.
- Treat CKC convenience-driven extras as requirement evidence; classify each as folded, dependency, deferred, conflict, or operator-decision-needed.
- Merge overlapping CKC and Atelier/Lens goals through explicit overlap rows instead of letting either source erase the other.
- Surface conflicts explicitly; resolve through layered scope or supersession, not silent deletion.
- Treat CKC code/spec/taskboard as prototype evidence and requirement source, not as Handshake runtime architecture.
- Rebuild in Handshake-native stack: Rust/Tauri command boundary, PostgreSQL authority, EventLedger lineage, artifact store, validation/promotion gates, Yjs/CRDT only where collaboration needs it, React as projection after contracts are stable.
- Reject CKC SQLite/Electron/localhost-intake for Handshake in any form. SQLite is not accepted in runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, or product paths.

## Preserved Source Paths

### CKC Source Paths

- `D:/Projects/LLM projects/CastKit-Codex/CKC_main`
- `D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/spec/CastKit_Codex_Spec_v00.075.md`
- `D:/Projects/LLM projects/CastKit-Codex/CKC_GOV/taskboard/TASK_BOARD.md`

### Handshake Source Paths

- `.GOV/task_packets/stubs/WP-1-Atelier-Lens-v2.md`
- `.GOV/task_packets/stubs/WP-1-Photo-Studio-v2.md`
- `.GOV/task_packets/stubs/WP-1-Atelier-Collaboration-Panel-v1.md`
- `.GOV/task_packets/stubs/WP-1-Lens-Extraction-Tier-v1.md`
- `.GOV/task_packets/stubs/WP-1-Lens-ViewMode-v1.md`
- `.GOV/task_packets/stubs/WP-1-Stage-Media-Artifact-Portability-v1.md`
- `.GOV/task_packets/stubs/WP-1-Stage-ASR-Transcript-Lineage-v1.md`
- `.GOV/task_packets/stubs/WP-1-Studio-Runtime-Visibility-v1.md`
- Adjacent Loom, media downloader, ASR, screenshot, visual-debugging, artifact-foundation, and structured-collaboration stubs inventoried in `handshake-stub-preservation-map.json`.

## Capability Clusters To Carry Forward

- Media viewer / DAM.
- Character sheets and templates.
- Inbox / intake sorter.
- Collections and contact sheets.
- Sidecars, versions, archive/restore, and recovery manifests.
- PoseKit / OpenPose / identity profile lineage.
- ComfyUI bridge, workflow receipts, and replay.
- Automation, model manual, leases, command map, screenshots, and debug captures.
- Search, tags, links/backlinks, similarity, palettes, and duplicates.
- Exports, backups, share packs, web portfolios, and LLM packs.

## Conflicts To Resolve, Not Drop

- CKC SQLite, FTS5, runtime DDL, and SQL translation are external source evidence only; no SQLite artifact, fixture, test, compatibility path, import path, fallback, cache, example, harness, or temporary adapter may be brought into Handshake.
- CKC Electron IPC and BrowserWindow shell are source evidence only; Handshake runtime uses Tauri/Rust boundaries.
- CKC localhost intake is source evidence only; Handshake needs a typed integration endpoint or artifact proposal path.
- CKC product strings (`CKC`, `CastKit Codex`, `CastKitCodexBridge`) must become Handshake/Atelier/Lens namespaces.
- Existing sparse stubs (`WP-1-Atelier-Lens-v2`, `WP-1-Photo-Studio-v2`) carry important Markdown intent even when their machine contracts are thin.
- Validated/superseded stubs may still be inherited requirements or evidence baselines.

## Corrected Runway

### Step 1: `WP-1-Atelier-Lens-Consolidation-v1`

Goal: produce the single preservation-first Atelier/Lens consolidation packet before any CKC rebuild stubs are created.

Scope:
- Preserve all existing Atelier/Lens/Photo/Studio/media/Loom/artifact/visual-debug stub goals.
- Fold CKC into Atelier/Lens as an evolved sibling source with overlapping intent.
- Build the Atelier/Lens versus CKC overlap matrix.
- Build the CKC evolved-feature and convenience-driven requirement register.
- Define conflict matrix, namespace migration, no-loss acceptance, and no-SQLite tripwires.
- Produce source-backed rows that make later CKC rebuild stubs possible without rereading all source material.

Not in scope:
- Product runtime implementation.
- UI rebuild.
- CKC rebuild/kernel/vertical-slice stubs.
- CKC feature parity claims before research and consolidation.

Microtask bucket target: 60+ execution microtasks when activated as an official consolidation packet.

Core buckets:
1. CKC source inventory.
2. CKC test inventory.
3. CKC spec requirement extraction.
4. CKC taskboard status extraction.
5. Handshake stub preservation map.
6. Prior packet and supersession scan.
7. Namespace migration matrix.
8. Runtime conflict matrix.
9. SQLite absolute rejection and no-test/no-fixture tripwire plan.
10. Electron rejection and Tauri mapping.
11. Product-output path hygiene plan.
12. Character/sheet requirement register.
13. Media/DAM requirement register.
14. Intake requirement register.
15. Collections/contact-sheet requirement register.
16. PoseKit/OpenPose requirement register.
17. ComfyUI requirement register.
18. Automation/manual/debug requirement register.
19. Search/tag/similarity requirement register.
20. Export/backup/share-pack requirement register.
21. Existing Handshake stub carry-forward matrix.
22. Atelier/Lens versus CKC overlap matrix.
23. CKC evolved-feature and convenience-driven requirement register.
24. Red-team scenarios and controls.
25. Validator acceptance model.
26. Official packet handoff draft.

### Deferred Packet Family: `WP-1-Atelier-Lens-CKC-Kernel-v1`

Status: do not create or activate this stub until `WP-1-Atelier-Lens-Consolidation-v1`, CKC greenroom review, CKC research, and conflict classification are complete.

Goal: implement the Handshake-native CKC kernel without a full GUI.

Scope:
- Rust domain modules for character, sheet, media, intake, collection, sidecar, PoseKit metadata, ComfyUI receipt, search projection, export manifest, and automation command catalog.
- PostgreSQL migrations and typed repository boundaries.
- EventLedger events for mutation, import, artifact proposal, validation, export, ComfyUI run, and recovery.
- Tauri command facade and tests.

Not in scope:
- Full dockable GUI.
- Full PoseKit calibration UI.
- Full ComfyUI generation UI.
- CKC direct runtime dependency.

Microtask bucket target: 80-120 execution microtasks when activated.

Core buckets:
1. Crate/module skeleton.
2. Domain ID types.
3. Character profile model.
4. Public/internal character ID split.
5. Sheet template parser contract.
6. Sheet value parser contract.
7. Sheet validation error model.
8. Sheet version append-only model.
9. Media asset model.
10. Media hash/provenance model.
11. Media review/status model.
12. Media tag/rating/favorite model.
13. Sidecar model.
14. Archive/restore/recovery model.
15. Intake batch model.
16. Intake item state machine.
17. Collection model.
18. Contact sheet manifest model.
19. Moodboard reference model.
20. PoseKit rig metadata model.
21. OpenPose sidecar contract.
22. Identity profile contract.
23. ComfyUI workflow receipt.
24. ComfyUI run lineage.
25. Workflow spec registry.
26. Search projection contract.
27. Link/backlink model.
28. Similarity/palette projection hooks.
29. Export manifest model.
30. Backup manifest model.
31. Automation command catalog.
32. Automation lease/session model.
33. Model manual contract.
34. EventLedger event definitions.
35. Artifact store integration.
36. Validation/promotion hook points.
37. Tauri command facade.
38. Postgres migrations.
39. Fixture import harness.
40. CKC parity test harness.

### Deferred Packet Family: `WP-1-Atelier-Lens-CKC-Vertical-Slice-v1`

Status: do not create or activate this stub until `WP-1-Atelier-Lens-Consolidation-v1`, CKC greenroom review, CKC research, and the CKC Kernel packet contract exist.

Goal: make the new kernel useful quickly through a narrow end-to-end slice.

Scope:
- Import or create a character.
- Attach media.
- Parse and version a character sheet.
- Run intake for a small batch.
- Create a collection/contact-sheet manifest.
- Record a ComfyUI recipe/run receipt as lineage.
- Show enough projection/API output for later UI work.
- Validate no-data-loss, no-silent-rewrite, and replayability.

Not in scope:
- Full production GUI.
- Full CKC parity.
- Advanced PoseKit calibration.
- Batch ComfyUI orchestration.

Microtask bucket target: 60-90 execution microtasks when activated.

Core buckets:
1. Vertical-slice fixture set.
2. Character create/import command.
3. Character read/projection command.
4. Sheet parse command.
5. Sheet diff/apply command.
6. Sheet version projection.
7. Media attach command.
8. Media hash/provenance test.
9. Media missing-file diagnostic.
10. Intake create batch.
11. Intake classify item.
12. Intake accept/pending/reject transitions.
13. Collection create command.
14. Contact-sheet manifest generation.
15. Sidecar hide/show projection.
16. OpenPose sidecar fixture ingest.
17. PoseKit metadata projection.
18. ComfyUI recipe receipt command.
19. ComfyUI run lineage event.
20. Export manifest projection.
21. Search/tag smoke projection.
22. Automation command list projection.
23. Screenshot/debug evidence inheritance check.
24. End-to-end replay and validation bundle.

## Existing Stub Consolidation Strategy

- `WP-1-Atelier-Lens-v2`: source material for CKC Greenroom and CKC Kernel; preserve role claiming, SceneState, ConflictSet.
- `WP-1-Photo-Studio-v2`: source material for CKC Kernel and Vertical Slice; preserve skeleton, thumbnails, recipes.
- `WP-1-Atelier-Collaboration-Panel-v1`: keep as validated/parallel baseline; reuse selection-scoped patching and provenance.
- `WP-1-Lens-Extraction-Tier-v1`: fold into CKC Kernel as Lens runtime parameter and trace field.
- `WP-1-Lens-ViewMode-v1`: keep as validated baseline; enforce projection-only filtering in CKC projections.
- `WP-1-Stage-Media-Artifact-Portability-v1`: fold into CKC Kernel artifact lineage and export manifest foundation.
- `WP-1-Stage-ASR-Transcript-Lineage-v1`: carry into later media intelligence; do not block first CKC character/media slice.
- `WP-1-Studio-Runtime-Visibility-v1`: fold into CKC Kernel event/projection/DCC traceability.
- Loom/media/ASR/video stubs: carry as adjacent media intelligence path; do not let them redefine CKC kernel identity.
- Screenshot/visual debugging stubs: inherit through Kernel visual evidence surfaces; do not reimplement unless coverage is missing.

## Activation Recommendation

Activate `WP-1-Atelier-Lens-CKC-Greenroom-v1` first. It should be massive but non-product-code: its output is the official no-loss requirements register, packet decomposition, and acceptance gates. Then activate `WP-1-Atelier-Lens-CKC-Kernel-v1` for Rust/Postgres/EventLedger implementation. Use `WP-1-Atelier-Lens-CKC-Vertical-Slice-v1` immediately after the kernel has enough contracts to support a useful character/media/ComfyUI lineage workflow.
