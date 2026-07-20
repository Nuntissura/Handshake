---
file_id: stage-product-topology-and-active-wp-migration
file_kind: reference-topology-supersession-and-optional-data-import-analysis
updated_at: "2026-07-19"
status: current-state-inspected-supersession-locked
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-product-topology" status="current-state-inspected-supersession-locked" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage product topology, shared-system reuse, and legacy Stage supersession

## Inspection basis

The protected product worktree and the three active development worktrees were inspected read-only on 2026-07-19. Paths below are repository-relative sibling-worktree snapshots, not permission to edit those worktrees from this planning lane. Before implementation, rebase the map to the operator-approved integration baseline and record exact commit SHAs.

## Reuse current shared authorities rather than duplicate

This reuse table applies to non-Stage Handshake authorities deliberately selected by the current plan. It does not grandfather any prior Stage implementation, schema, adapter, connector, pane, route, or mockup. Old Stage assets are evidence or optional code-level salvage only and must satisfy the current contract without compatibility concessions.

| Concern | Current product evidence | Stage disposition |
|---|---|---|
| Canonical artifacts | `../handshake_main/src/backend/handshake_core/src/ace/mod.rs` defines structured `ArtifactHandle`. | Reuse exactly; no Stage string handle or byte-store authority. |
| Workflow jobs | `../handshake_main/src/backend/handshake_core/src/workflows.rs`. | Stage initiates/observes versioned jobs; background work routes through the existing workflow surface. |
| Durable event authority | `../handshake_main/src/backend/handshake_core/src/flight_recorder/event_ledger.rs`. | State mutation and durable events use the canonical EventLedger/outbox contract; no Stage event store. |
| Diagnostic evidence | `../handshake_main/src/backend/handshake_core/src/flight_recorder/` and `diagnostics/`. | Extend typed events, spans, health, and support-bundle projections. |
| Process ownership/recovery | `../handshake_main/src/backend/handshake_core/src/process_ledger/`. | Reuse for Handshake-spawned Servo/CEF/CfT helpers after WP-1 validation; vendor-owned WebView2 processes use official environment events unless a proven integration says otherwise. |
| PostgreSQL/retention | `../handshake_main/src/backend/handshake_core/src/storage/postgres.rs` and `storage/retention.rs`. | Add Stage tables/migrations through existing storage patterns; do not treat browser profile files as canonical records. |
| Downloader/intake | `../handshake_main/src/backend/handshake_core/src/atelier/downloader.rs`, `atelier/intake.rs`, and `atelier/action_receipt.rs`. | Reuse versioned downloader/intake/receipt paths; Stage supplies source/session context and canonical artifacts. |
| Search/Loom | `../handshake_main/src/backend/handshake_core/src/loom_search/mod.rs` plus native `search_rail.rs` and `loom_graph.rs`. | Extend indexes/projections and UI patterns; Loom retains knowledge-relationship authority. |
| Native events | `../handshake_main/src/frontend/handshake_native/src/event_bus.rs`. | Stage emits/consumes typed native projections without making the UI bus durable authority. |
| User/model manual | `../handshake_main/src/backend/handshake_core/src/user_manual/`. | Register every stable Stage action, setting, workflow, error, recovery, and evidence route. |
| Diagnostics bundle | `../handshake_main/src/backend/handshake_core/src/diagnostics/bundle_manifest.rs` and `product_anchor_matrix.rs`. | Extend the existing redacted bundle and anchor topology. |

## Proposed future ownership map

Exact file creation remains a later implementation decision. The future WP must assign one merge owner for each shared integration surface.

| Surface | Owner | Shared/consumer edges | Forbidden duplication |
|---|---|---|---|
| Stage records, lifecycle, commands, queries, adapter trait | Stage backend domain | PostgreSQL, EventLedger, native projections | No engine-native IDs as durable authority. |
| Native Stage module and browser chrome | Stage native UI | shell registry, AccessKit/UIA, visual debugger | No second authoritative Stage pane. |
| Browser adapters | Stage adapter modules | WebView2, Servo, optional CfT/CEF workers | Vendor types cannot escape adapter boundary. |
| Model browser actions | Stage control service | WP-1 ToolGate/model lanes/process/telemetry | Stage cannot own provider/model promotion. |
| Captures and exports | `StageCaptureCoordinator` (Stage Capture) | Shared ArtifactStore, Export/Materialize, Downloader, ASR | No inline Stage byte authority, raw-path authority, shared-store scan, or Stage-specific artifact-handle encoding. |
| Editor-to-Stage public integration | New Stage public route selected by the current plan | WP-12 native editors or later editor consumers | No legacy WP-12 Stage route, pane, wire type, alias, or state authority. |
| Project intake | Stage orchestration adapter | Atelier/Lens/CKC public schemas | No direct consumer access to Stage tables/profiles. |

## Canonical worksurface decision

The WP-12 worktree currently contains two different Stage concepts:

- `ModuleId::Stage` in `src/frontend/handshake_native/src/module_switcher.rs`, whose current default tab is `PaneType::FontManager`;
- `StagePane`/`StagePaneMount` in `stage_pane.rs`, `pane_registry.rs`, and `editor_pane_factories.rs`, used as the editor route/capture/embed-back client.

The operator-locked production topology is:

1. the full native Stage module is the canonical browser workspace;
2. the editor `StagePane` and current `ModuleId::Stage` placeholder are superseded rather than retained as compatibility clients;
3. `ModuleId::Stage` no longer points at Font Manager once Stage implementation owns the module;
4. if the current requirements retain editor-to-Stage and embed-back workflows, they use a newly specified public contract for editor context, causal action, idempotency, stale-target, undo/redo, and receipts without legacy aliases;
5. no pane or bottom-panel client owns independent Stage persistence.

This decision no longer requires a later worksurface choice. Official MT allocation must implement the canonical module and explicitly retire or replace the older WP-12 Stage UI surfaces.

</topic>

<topic id="stage-active-wp-migration" status="current-state-inspected-supersession-locked" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Active-WP integration, legacy Stage supersession, and optional data import

## WP-1 model orchestration

WP-1 remains `IN_PROGRESS`; current dirty source and validator failures are not a production baseline. Stage can define consumer contracts and fakes now, but production claims depending on process termination/reaping, ToolGate, promotion, capability projection, EventLedger, Flight Recorder, or Argus must bind to a later validated WP-1 commit.

Ownership invariant:

- Stage owns desired browser state, browser-specific leases, adapter actions, browser observations, and browser receipts;
- WP-1 owns model lanes, provider/model selection, ToolGate, shared process authority, cancel/reap, promotion, and shared telemetry;
- the versioned integration must specify which store owns the browser-control lease, fencing token, expiry, heartbeat, and atomic reconciliation for browser crash, lane cancellation, and operator takeover.

Required entry gate fields are `dependency_wp`, `required_contract_version`, `baseline_commit`, `validator_state`, `required_green_checks`, `fake_contract_version`, `unavailable_behavior`, `revalidation_trigger`, and `evidence_ref`.

## WP-12 Stage prototype inventory

Inspected behavior available only as evidence:

- `interop.route-to-stage` for document, selection, canvas node, and Atelier item;
- exact-byte SHA-256/provenance validation;
- idempotency, causal action, stale-target, target-gone, busy, persistence-pending, and compensation behavior;
- `stage_capture` presentation and capture/embed-back status.

None of these names or shapes is a compatibility requirement. Current Stage requirements independently decide which outcomes survive and define their new IDs, schemas, UI, and evidence.

Transitional implementation to replace:

- `src/backend/handshake_core/src/api/stage.rs` caps content at 16 KiB and exposes `StageArtifactRefWire`;
- `src/backend/handshake_core/src/storage/stage_artifacts.rs` stores bytes in `stage_capture_artifacts` and uses capability `stage.jobs.enqueue`;
- those shapes conflict with canonical ArtifactStore authority, structured `ArtifactHandle`, Workflow Engine profiles, and future Stage tool/method IDs.

## Migration-number collision

The active worktrees contain same-number, different-purpose migrations:

| Number | WP-12 | WP-1 | CKC |
|---|---|---|---|
| `0341` | `stage_capture_artifacts` | `model_lane_cloud_projection_consent` | `atelier_intake_item_metadata` |
| `0346` | `stage_capture_runtime_contract` | `user_manual_wp1_mt015_cloud_access_origin` | none inspected |
| `0348` | `stage_capture_integrity_and_canvas_provenance` | `model_runtime_registry` | none inspected |
| `0349` | `stage_canvas_provenance_json_string_types` | `model_lane_stable_anchor` | none inspected |

WP-12 migration `0348` is not local to one Stage table. It installs Stage-compensation/provenance guards across Loom canvas placements, document backlinks, FEMS memory proposals, quick-switcher recents, context-bundle items, knowledge sources, Loom AI suggestions, and Loom edge references. Renumbering alone is insufficient; the legacy graph must be inventoried and removed or replaced across every affected domain. Only designated real operator data is imported into the current model and rollback-tested.

## Required supersede/import-if-needed/remove plan

1. Freeze an approved integration baseline and inventory legacy Stage readers, writers, migrations, guards, UI entry points, tests, mock data, and whether any real operator data exists outside tests.
2. Classify every older Stage surface as `REMOVE`, `REPLACE`, `SALVAGE_CODE_ONLY`, or `ONE_WAY_DATA_IMPORT`; no compatibility disposition is implicit.
3. Reserve new migration numbers from the post-merge canonical sequence; never reuse the colliding numbers.
4. Introduce only the current canonical Stage/ArtifactStore schemas and public contracts. Do not create compatibility views, legacy response adapters, or alias routes by default.
5. Stop legacy writes before cutover. If real operator data exists, import inline bytes and required provenance into ArtifactStore in bounded idempotent chunks, verifying counts and SHA-256 before canonical handles are committed.
6. Replace or remove WP12 cross-table Stage guards so every retained reference targets the current canonical model. Historical field names survive only inside an isolated import reader when needed.
7. Switch selected non-Stage consumers directly to the new current Stage contract. Dual-read, dual-write, canonical-write-plus-legacy-projection, and legacy compatibility windows are forbidden unless separately approved for a proven non-Stage external constraint.
8. Remove `stage.jobs.enqueue`, `StageArtifactRefWire`, `/stage/artifacts`, `stage_capture_artifacts`, `StagePane`, and legacy Stage aliases/connectors after any approved data import and consumer cutover.
9. Rehearse clean removal plus, when real data is designated for retention, interrupted import, restart/resume, duplicate input, corrupt/missing bytes, partial cross-table references, count/hash reconciliation, rollback-before-cutover, and safe-mode/export after cutover.
10. Prove repository and runtime scans find no legacy Stage reader, writer, alias, route, pane, adapter, connector, schema authority, or mock-backed production path.

Rollback boundaries:

- before canonical cutover: roll back the new release without reactivating old Stage authority; retain a read-only export/import escape path where real data exists;
- during one-way import: stop safely and resume by cursor/idempotency record;
- after canonical-only writes begin: do not restore legacy byte authority; rollback uses the previous current-Stage release, forward-fix, or read-only safe mode plus export;
- after legacy removal: downgrade to an older Stage implementation is refused unless an explicit operator-approved reverse data transformation with loss report has passed.

## CKC boundary

CKC remains a downstream consumer, not a Stage implementation dependency. The Stage-to-project intake fixture must cover version, idempotency, partial item dispositions, causation, recursion-loop limits, and route-back. CKC receives public artifact/lineage envelopes only and cannot access Stage tables, profiles, cookies, engine-native identifiers, or model-control leases.

</topic>
