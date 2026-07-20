---
file_id: stage-active-wp-compatibility
file_kind: reference-integration-ownership-and-legacy-stage-supersession-plan
updated_at: "2026-07-19"
status: legacy-stage-supersession-locked
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-active-wp-snapshot" status="verified-snapshot" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Active work-packet snapshot

This is a dated planning snapshot, not a validation verdict. All three development worktrees were materially dirty when inspected, so a committed verdict does not prove the present working tree. Reverified 2026-07-20 against the typed packet contracts: all three lifecycle statuses remain `IN_PROGRESS`; the rows below still hold.

| Operator label | Canonical packet | Typed status snapshot | Stage significance |
|---|---|---|---|
| WP CKC | `WP-CKC-posekit-overhaul` | `IN_PROGRESS`; packet records whole-WP technical `PASS_AT_HEAD`, but merge/main compatibility are withheld | Active Atelier/CKC/PoseKit/Ingest consumer and route-to-Stage peer. |
| WP 12 | `WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1` | `IN_PROGRESS`; V1 packet evidence contains both PASS and FAIL rows, with V2 repair work not yet holding a final validator verdict | Owns native editor routing and embed-back UX; contains a Stage prototype that overlaps future Stage authority. |
| WP 1 model orchestration | `WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1` | `IN_PROGRESS`; packet contains unresolved V2 failures | Dexterity/model-lane, ToolGate, process recovery, promotion, telemetry, and diagnostic provider. |

Traceability/build-order projections contain stale or incomplete rows for these packets. Their typed packet contracts and validator/runtime receipts remain the planning inputs. None is a passed Stage dependency merely because code or a prior packet verdict exists; `dependency-gates.yaml` records the fail-closed entry state.

</topic>

<topic id="stage-wp12-compatibility" status="supersession-locked" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# WP-KERNEL-012 Stage supersession and prototype disposition

WP12 contains a note/selection/canvas-to-Stage and capture/embed-back prototype. The current Stage direction supersedes its Stage-specific implementation and contracts. A comparable editor workflow remains only where the current Stage requirements independently select it, and it must be specified against new Stage public contracts rather than inherited from WP12.

## Existing prototype inspected

- `../wtc-native-editors-v1/src/frontend/handshake_native/src/stage_pane.rs`
- `../wtc-native-editors-v1/src/frontend/handshake_native/src/interop/stage_interop.rs`
- `../wtc-native-editors-v1/src/backend/handshake_core/src/api/stage.rs`
- `../wtc-native-editors-v1/src/backend/handshake_core/src/storage/stage_artifacts.rs`
- `../wtc-native-editors-v1/src/backend/handshake_core/migrations/0341_stage_capture_artifacts.sql`
- `../wtc-native-editors-v1/src/backend/handshake_core/migrations/0346_stage_capture_runtime_contract.sql`
- `../wtc-native-editors-v1/src/backend/handshake_core/migrations/0348_stage_capture_integrity_and_canvas_provenance.sql`
- `../wtc-native-editors-v1/src/backend/handshake_core/migrations/0349_stage_canvas_provenance_json_string_types.sql`

The prototype supports editor-originated `Document`, `Selection`, `CanvasNode`, and `AtelierItem` payloads, an `interop.route-to-stage` command, a `stage_capture` embed kind, correlation IDs, typed blockers, exact-byte reads, idempotency, SHA-256 checks, and provenance refusal. These are inspected evidence only. They create no compatibility, naming, schema, route, adapter, connector, pane, or reuse obligation.

It is not the planned Stage browser product. It also creates a dedicated `stage_capture_artifacts` table and `StageArtifactRefWire`, stores inline bytes with a 16 KiB limit, hardcodes `stage.jobs.enqueue` as a capability, and exposes Stage-specific artifact routes. Those shapes conflict with current canonical ArtifactHandle/ArtifactStore authority, capability/tool separation, and job compile-down requirements.

The removal/data-disposition problem is semantic as well as numerical. WP12 uses `0341`, `0346`, `0348`, and `0349` while WP-1 uses different migrations with all four same numbers and CKC uses another `0341`. WP12 `0348` installs Stage-reference/compensation guards across Loom canvas, document backlinks, FEMS, quick switcher, context bundles, knowledge, and Loom edges. The later plan must remove or replace the full legacy Stage graph, not merely renumber migrations. If real operator data exists, a one-way import into the new canonical model must preserve that data without preserving the old APIs or runtime authority.

The native WP12 shell also contains `ModuleId::Stage` with `FontManager` as its current default tab and a separate `StagePane`/bottom-panel client. The operator has resolved this topology: the new full Stage module is canonical, and both older Stage placeholders are superseded. A future editor entry point may open the canonical Stage module through a newly specified public route, but the old pane is not retained by default.

## Recommended disposition

| Prototype element | Disposition | Owner after Stage overhaul |
|---|---|---|
| `interop.route-to-stage` editor command and route UX | Supersede. If the current plan selects editor-to-Stage routing, implement a new public Stage route without legacy aliases. | new Stage public contract + editor consumer |
| `Document` / `Selection` / `CanvasNode` / `AtelierItem` route kinds | Historical design evidence only. Current source-reference and ArtifactHandle contracts are designed from current requirements. | new Stage public contract |
| Correlation IDs, typed blocker state, liveness recheck, provenance refusal | Re-derive and prove where required; do not inherit the WP12 wire contract. | new Stage/editor contract |
| `stage_capture` embed kind | Supersede. A current editor presentation discriminator, if needed, receives a new canonical contract. | editor projection selected by current Stage |
| `StagePane` and `ModuleId::Stage` FontManager placeholder | Supersede and remove/replace during integration. They are not compatibility clients or product authority. | canonical native Stage module |
| `/stage/artifacts` routes, `StageArtifactRefWire`, `stage_capture_artifacts` table | Supersede. Import selected real operator data once if present; test/mock data and legacy authority need not survive. Stage uses `StageCaptureCoordinator` plus shared ArtifactStore and canonical opaque handles. | Stage migration owner + shared Artifact System owner |
| `stage.jobs.enqueue` capability check | Remove and replace with current registered capability/tool/method contracts; no legacy alias is presumed. | Capability Registry + Stage |
| WP12 Stage EventLedger/Flight Recorder proof | Historical evidence only; re-prove through the canonical Stage event vocabulary and three-tier diagnostics. | Stage integration suite |

## Required new-contract tests

1. Route a note, selection, canvas node, and Atelier item to an existing or newly opened Stage tab without losing source identity.
2. Capture in Stage and embed a canonical ArtifactHandle back into the originating note/canvas with hash and manifest provenance.
3. Reject stale/deleted editor targets and provide a recoverable alternate destination.
4. Maintain route/capture correlation through restart and retry without duplicate embeds.
5. Prove the editor never reads Stage private tables or renderer-specific IDs.
6. Prove every command is AccessKit/model addressable and does not steal OS focus during automated tests.

</topic>

<topic id="stage-ckc-compatibility" status="verified-gap-proposed-contract" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# CKC/Atelier compatibility

The active CKC packet owns the native Atelier/CKC/PoseKit/Ingest production workflow. Stage remains separate and supplies acquired/captured assets plus source lineage; CKC/Atelier supplies intake disposition, production artifacts, and route-back actions.

## Ownership boundary

| Stage owns | CKC/Atelier owns |
|---|---|
| Browser/source session, tab, navigation, capture, download coordination, and source provenance | Project intake batch, disposition, collection, character/media/pose/facial production state |
| Shared canonical opaque ArtifactHandle delivery plus StageCapture/source-artifact lineage | Consumption of ArtifactHandles through its public intake contract |
| Stage-side route/open and browser context | Atelier UI, CKC internal identifiers, PoseKit and downstream production outputs |
| Browser credential/session handoff to approved acquisition jobs | No direct access to Stage cookies, browser profile folders, or renderer state |

## Required interface

The later contract should define a canonical Stage-to-project intake request containing at least:

- idempotency, correlation, and source-action IDs;
- workspace/project target;
- canonical ArtifactHandle list;
- source URL/origin, capture timestamp, renderer/adapter provenance, and capture limitations;
- Media Downloader/ASR lineage handles when present;
- requested intake mode and operator/model actor;
- no raw Stage filesystem path, cookie, DOM session, or private table reference.

Direct product inspection resolves the earlier uncertainty: a Stage-ready public artifact-batch adapter does not currently exist. `atelier/intake.rs` can open batches and register `NewIntakeItem` rows using `source_path`, file name, byte length, and content hash; `api/atelier.rs` exposes batch creation, item queries, and classifications, while `atelier/media.rs` already understands ArtifactStore-backed media refs. Stage must not pretend this is a reusable public adapter or invent a CKC contract inside Stage.

The required addition is a consumer-owned versioned Atelier/CKC intake endpoint that accepts shared opaque ArtifactHandles plus source/capture/job correlation, causation, idempotency, intended consumer/mode, and bounded per-item payload metadata. It returns complete per-item accept/reject/defer/duplicate/failed/retryable dispositions, batch progress, and stable result refs. It cannot require unmanaged source paths or grant the consumer access to Stage tables, profiles, cookies, engine IDs, or renderer state. `STAGE-DEP-ATELIER-CKC-INTAKE` is blocked until that contract and its restart/partial-failure/loop tests pass.

## Loop and failure controls

- A CKC item routed to Stage and then re-ingested must preserve causation and reject accidental recursive intake loops.
- Partial intake returns per-item dispositions; Stage does not call the whole batch successful from a UI count.
- Closing Stage cannot cancel a detached CKC or Media Downloader intake job.
- Stage must surface downstream failure without mutating CKC authority directly.

</topic>

<topic id="stage-wp1-dexterity-compatibility" status="proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# WP-1 Dexterity/model-orchestration compatibility

Stage needs browser-specific observation, action, lifecycle, and control leases. It does not need a second model runtime or agent coordinator.

## Ownership boundary

| Stage standardizes | Reuse from WP-1 Dexterity |
|---|---|
| Durable `StageSession`/`StageTab` identity and renderer generation | `ModelLaneRun`, `ModelLane`, `ModelLaneMessage` |
| Navigation, lifecycle, semantic/visual/network observation, input actions, focus, restore, and bulk-tab semantics | ContextBundle and canonical artifact-reference transfer |
| Browser-engine capability negotiation and unsupported results | Provider consent, model selection, and ToolGate |
| Browser action/observation receipts and postconditions | PostgreSQL/EventLedger, Flight Recorder, Argus, internal diagnostics, Palmistry |
| Browser-control/operator-takeover leases | ProcessOwnershipLedger and terminate/reap/recovery machinery |
| Canonical tab registry and resource scheduler | PromotionGate and CRDT proposal paths for model-authored durable changes |

## Required correlation contract

Never overload `session_id`. A model-driven Stage action must correlate distinct values:

- `model_session_id`;
- `model_lane_run_id` and `model_lane_id`;
- `stage_session_id` and stable `stage_tab_id`;
- Stage record revision;
- renderer adapter/version/capability manifest;
- engine instance/generation/context/navigation IDs;
- action/attempt/idempotency/causation/correlation/trace IDs;
- ToolGate decision, focus policy, deadline, pre-observation hash, and terminal/postcondition verdict.

A browser crash or timeout with uncertain side effect becomes `OUTCOME_UNKNOWN_RECONCILE_REQUIRED`; Dexterity may not blindly retry a non-idempotent action.

## Production dependency gates

Stage can specify this integration now, but cannot claim production completion while WP-1 still has unresolved attached-sandbox ownership, terminate/reap recovery, stop-reporting, stale/ABA claim, capability-selection, projection-integrity, or diagnostic-proof failures relevant to the invoked path.

The integration suite must run at least two concurrent model lanes plus operator interaction without hidden peer channels, silent overwrite, focus theft, deadlock, starvation, orphaned browser/model processes, or authority promotion outside PromotionGate. `STAGE-DEP-WP1-MODEL-ORCHESTRATION` remains blocked until exact contract versions/baseline commit are bound and relevant validator outcomes are PASS rather than implementer-ready claims.

</topic>

<topic id="stage-media-downloader-compatibility" status="verified-gap-proposed-contract" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Media Downloader compatibility

Current product inspection shows a split rather than one ready production boundary. `atelier/downloader.rs` contains records/contracts, while the executing v0 workflow is in `workflows.rs`. The workflow reads `ArtifactHandle.path`, uses a legacy `.handshake/gov/media_downloader_sessions.json` Stage-session registry, and implements a separate streaming artifact writer. Those shapes conflict with the newly locked opaque shared handle, scoped credential lease, shared streamed ArtifactStore, and Stage-session ownership boundaries.

Before Stage production integration, the Downloader owner must converge the versioned request/control/result contract and exact implementation baseline; consume `CredentialLeaseRef` rather than a legacy Stage session registry or routine raw cookie file; use the canonical opaque shared ArtifactHandle and shared streamed ArtifactStore ingest; and preserve pause/resume/cancel/retry/partial/captions/ASR/renderer-kill behavior. External materialization is an explicit structured policy, not artifact identity. `STAGE-DEP-MEDIA-DOWNLOADER` remains blocked until this evidence passes.

</topic>

<topic id="stage-active-wp-integration-risks" status="active" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Integration risks and hardening

| Risk | Failure scenario | Mitigation and proof |
|---|---|---|
| Two Stage domain models | WP12 table/routes and new Stage kernel both become authoritative. | Supersede and remove the WP12 Stage implementation; allow one-way real-data import only; prove one canonical ArtifactHandle and event vocabulary with no direct-table consumer access. |
| Migration collision and semantic cross-table coupling | WP12 and WP1 collide on `0341/0346/0348/0349`, CKC collides on `0341`, and WP12 `0348` guards Stage references across multiple domains. | Freeze integration baseline; allocate new numbers; inventory real data and readers/writers; stop legacy writes; import only retained data; replace/remove guards and legacy surfaces; prove no runtime dependency remains. |
| Dirty-worktree false confidence | Old validator verdict is cited for current uncommitted code. | Revalidate exact heads and dirty diffs; record commit, diff hash, database baseline, and test artifacts. |
| Cross-WP cyclic workflow | Stage routes to CKC/editor, which routes back and duplicates intake/embed. | Causation chain, idempotency, loop budget, duplicate detection, and explicit operator-visible reconciliation. |
| Authority bleed | Editor or CKC client writes Stage tables or browser state directly. | Public typed APIs only, contract tests, denied-path tests, and storage ownership checks. |
| Orchestration duplication | Stage creates its own model sessions, tool gate, or process ledger. | Require WP-1 IDs/receipts for model-driven actions and fail startup/invocation when required capabilities are unavailable. |
| Shared artifact dependency is assumed ready | Stage buffers large media/PDF/WARC payloads or depends on conflicting handle encodings. | Pass the global handle and streamed-ingest gates; Stage treats handles as opaque and never implements a parallel store. |
| Missing CKC intake adapter is called existing reuse | Stage writes path-oriented consumer rows or declares success without per-item dispositions. | Require a new consumer-owned public ArtifactHandle batch endpoint and partial/restart/loop proof. |
| Downloader v0/v2 split leaks legacy Stage state | Downloader reads handle paths or a legacy Stage-session file and duplicates artifact streaming. | Bind one contract/baseline, scoped credential lease, opaque handle, and shared streamed-ingest implementation. |
| Branch-base contamination | Stage implementation starts from CKC/WP12/WP1 dirty branch assumptions. | Start from an operator-approved integration baseline; consume versioned interfaces, not branch-local private code. |

</topic>
