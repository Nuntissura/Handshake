---
file_id: ckc-atelier-lens-final-wp-stub-map
file_kind: final_stub_map
updated_at: 2026-05-16
wp_id: WP-1-Atelier-Lens-Consolidation-v1
status: draft_mt_suites_rebuilt_not_execution
---

# CKC Atelier/Lens Final WP Stub Map

<topic id="operator-deliverable" status="repaired" version="2" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

The operator requested no product coding. The immediate deliverable is planning: consolidate Atelier/Lens stubs, fold CKC into the same Handshake feature family, preserve intent and technical details, and keep the CKC fold-in to no more than three CKC-specific WP stubs before any official microtask generation.

Repair note: audit findings on 2026-05-16 found the first three-stub pass directionally correct but not preservation-complete. The three stubs were repaired to include missing old-stub payload, CKC app coverage, media/ASR/Loom details, Pose/ComfyUI schemas, visual/debug/manual/build diagnostics, and evidence-maturity gates. This map no longer means "ready for execution"; it means the repaired WP stubs are ready for operator/stub review before MT expansion.

Draft MT repair note: the earlier 20-item inline draft MT lists were a self-imposed limit and are retired. They have been replaced with fresh non-executable draft MT suites that must still go through refinement, USER_SIGNATURE, and official packet activation before becoming official MT contracts.

</topic>

<topic id="final-stubs" status="repaired" version="2" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Final Three Stub Buckets

1. `WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1`
   - Owns character sheets, templates, media/DAM, intake/inbox, collections/contact sheets, sidecars/versioning/recovery, search/tag/similarity, exports/backups/share-packs.
   - Folds Atelier/Lens core, Photo Studio, LensExtractionTier/ViewMode, Stage media artifact portability, Media Downloader v1/v2, ASR, Loom/archive, ArtifactStore, CKC data/library/docs/stories/moodboard/relationships/media review/import/export/reset features.

2. `WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1`
   - Owns PoseKit/OpenPose, identity profiles, ComfyUI workflow receipts, image-generation registration/replay, workflow registry, mechanical tool adapter boundaries, lineage.
   - Folds CKC PoseKit/ComfyUI evidence into Handshake Workflow Engine, Tauri/Rust/Python, ArtifactStore, EventLedger, Studio runtime, and tool governance, with concrete Rig/OpenPose/IdentityProfile/WorkflowReceipt fields and blocked/planned PoseKit rows preserved.

3. `WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1`
   - Owns model-facing manual, command catalog, sessions/leases/heartbeats, command logs, DCC/Locus/Flight Recorder projections, diagnostic bundles, structured state, visual evidence, and non-focus automation rules.
   - Folds CKC automation/manual/debugger work into Handshake's mechanical separation of LLM and execution, including visual-debug loop details, local LLM/AI tagging visibility, build/release/install evidence, and stale-doc/spec drift detection.

</topic>

<topic id="draft-microtask-suites" status="draft_non_executable" version="1" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Draft Microtask Suites

| Stub | Draft MT count | Draft suite path | Execution status |
|---|---:|---|---|
| `WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1` | 80 | `.GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Core-Data-Intake-v1/MT_SUITE.md` | non-executable planning only |
| `WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1` | 51 | `.GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Pose-ComfyUI-Pipeline-v1/MT_SUITE.md` | non-executable planning only |
| `WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1` | 66 | `.GOV/task_packets/stubs/draft_microtasks/WP-1-Atelier-Lens-CKC-Model-Workflow-Diagnostics-v1/MT_SUITE.md` | non-executable planning only |

Activation rule: do not convert these draft MTs into `.GOV/task_packets/<WP_ID>/MT-*.json` or `.md` files until refinement, USER_SIGNATURE, and official packet creation. Activation Manager must split any draft MT further if it touches unrelated files, crosses owner boundaries, or cannot be executed by a no-context local/small cloud model from that MT alone.

</topic>

<topic id="non-negotiables" status="repaired" version="2" wp="WP-1-Atelier-Lens-Consolidation-v1" updated_at="2026-05-16">

## Non-Negotiables

- No product code is implemented by these stubs.
- CKC is source evidence, not runtime architecture.
- Handshake stack translation is required: Tauri, Rust backend coordinator, Python AI orchestration where appropriate, React/TypeScript projections, PostgreSQL primary authority, Yjs/CRDT collaboration boundaries, ArtifactStore, EventLedger, Flight Recorder, Workflow Engine, AI Job, MicroTask, Locus, DCC, MCP/tool governance.
- LLMs do not execute product operations directly. They propose or invoke governed jobs/tool calls; Handshake runtime executes and records evidence.
- SQLite is forbidden in Handshake runtime, tests, fixtures, mocks, examples, fallbacks, cache, compatibility adapters, temporary harnesses, imports, exports, and product paths.
- Electron IPC, CKC localhost intake authority, CKC product namespace, and `.GOV` product outputs are rejected for runtime implementation.
- The old 20-MT inline draft lists are retired and must not be reused for activation.
- Official MT generation is still blocked until activation/refinement approval for the repaired three-stub family and draft MT suites.

</topic>
