---
file_id: stage-mvp-backend-and-data-contracts
file_kind: reference-architecture-plan
updated_at: "2026-07-19"
status: proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-backend-and-data-contracts" status="proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage backend and data contracts

## Planning outcome

Stage should have one Handshake-owned browser kernel and one product behavior contract. Servo is the target primary renderer. Chromium supplies the smallest first-usable and compatibility bootstrap, but it does not define Stage-domain storage, tabs, sessions, jobs, search, capture, policy, or model-control semantics.

This is intentionally not a coequal-backend product strategy. Shared interfaces exist to prevent bootstrap code from contaminating Stage and to allow Servo promotion, not to fund duplicate feature development.

## Proposed system shape

```text
Native Stage UI
  -> Stage command/query boundary
     -> TabCatalog + WindowRegistry
     -> TabLifecycleManager
     -> RendererSupervisor
        -> ServoAdapter (target primary)
        -> ChromiumBootstrapAdapter (minimum bounded subset)
     -> SessionProfileManager
     -> NavigationAndNetworkPolicy
     -> AgentObservationAndActionGateway
     -> StageCaptureCoordinator
        -> Workflow Engine / Mechanical Tool Bus
        -> Media Downloader v2
        -> ArtifactStore / Materialize / Export
        -> ASR and Atelier/Lens intake
     -> SearchAndContentIndex
     -> LoomProjectionAdapter
     -> TranslationCoordinator
     -> Diagnostics / Flight Recorder
```

The renderer adapter should remain narrow and capability-reported. A renderer may implement navigation, input, pixel output, DOM/script evaluation, accessibility snapshots, screenshots, request observation, download interception, permissions, storage hooks, and print behavior. Stage-domain behavior stays above that boundary.

## Responsibility map

### Stage-owned services

- `WindowRegistry`: per-window folder tree, ordering, saved views, and active selection.
- `TabCatalog`: durable tab identity, metadata, workflow state, relationships, and canonical queries.
- `TabLifecycleManager`: active/suspended/unloaded/archived decisions, live budget, exemptions, and transitions.
- `RendererSupervisor`: renderer creation, binding, crash/hang containment, health, and explicit engine provenance.
- `SessionProfileManager`: persistent/ephemeral profiles, cookies, cache, storage, permissions, and session isolation.
- `NavigationAndNetworkPolicy`: URL validation, redirects, private-network policy, request blocking, download routing, and Stage App boundaries.
- `AgentObservationAndActionGateway`: typed observations/actions, capability checks, attribution, stale-target rejection, receipts, and replay hooks.
- `StageCaptureCoordinator`: Stage Capture intent, source-revision validation, capture/download/import job correlation, normalized capture-part lineage, completion reconciliation, and downstream handoff through existing workflow and artifact systems. It owns no bytes, global manifests, hashing, retention, garbage collection, materialization, or raw paths.
- `SearchAndContentIndex`: metadata fuzzy search and captured-content full-text retrieval without waking tabs.
- `LoomProjectionAdapter`: references to LoomBlocks/tags/mentions/backlinks without copying Loom graph authority.
- `TranslationCoordinator`: local/cloud adapter selection, egress control, provenance, and derived artifacts.
- `StageDiagnostics`: renderer/process/job/session/policy/agent traces and bounded model-readable state.

These names are provisional and should be reconciled against inspected product-code topology before the work packet is finalized.

### Renderer-owned behavior

- parsing, layout, painting, compositing, page script execution, and renderer-local history mechanics;
- DOM/accessibility/network primitives the adapter can expose safely;
- page-level cookie/cache/storage primitives delegated through the Stage Session context;
- renderer-specific crash, hang, and capability signals;
- pixel and print/snapshot production primitives.

### Existing Handshake systems reused

- Workflow Engine and Mechanical Tool Bus for privileged/background execution;
- Flight Recorder for bounded decisions, events, receipts, and failures;
- ArtifactStore for immutable bytes/manifests and Materialize/Export for external paths;
- Media Downloader v2 for supported web-media acquisition;
- ASR for derived transcript artifacts and timing lineage;
- Loom for knowledge graph relationships;
- Atelier/Lens intake for project collections, review state, indexing, and consumer-specific projections.

## Proposed durable data model

The following are candidate records, not locked schema names.

### Window and folder records

- stable window ID, title, project context, selected tab, layout preferences, and timestamps;
- stable folder ID, parent folder ID, window ID, name, display color, ordering key, optional Loom tag/view reference, and archived state;
- saved view records contain a query/filter/sort definition, never a copied list of visible rows.

### Tab record

A durable tab record needs at least:

- stable tab ID and owning window/folder IDs;
- original URL, normalized URL, title, favicon/thumbnail ArtifactHandles, and last-known page language;
- bookmark, color label, reminder note, project reference, intended consumer, and workflow state;
- parent/discovery relation such as `opened_from_tab_id` plus optional ordered watch relation;
- lifecycle state and timestamps for opened, selected, suspended, unloaded, archived, watched, downloaded, and ingested;
- current Stage Session ID and renderer provenance for the last live binding;
- keep-live exemption reason and expiry where applicable;
- compact current workflow/capture/Loom status references; arbitrary artifact/job relationships live in normalized Stage capture/source-artifact link records rather than unbounded tab arrays;
- restoration target plus bounded history/snapshot references according to later retention decisions;
- revision/concurrency metadata so parallel agents cannot silently overwrite operator organization.

The live renderer handle, DOM node handles, process IDs, transient JavaScript objects, and secret cookie values must never be treated as durable tab state.

### Session record

- stable session ID, operator-visible name, persistence mode, project/agent scope, and renderer compatibility metadata;
- encrypted profile/storage location through a relocatable workspace configuration;
- cookie/cache/storage/service-worker/permission policy and usage summaries;
- creation, last-use, retention, clear, export, and destruction evidence;
- no raw secrets in general-purpose events, logs, search indexes, or tab records.

### Observation and action records

- actor/model identity, tab/session/window IDs, renderer and capability provenance;
- navigation/document revision and observation revision;
- bounded DOM/accessibility snapshot reference, screenshot ArtifactHandle, viewport, and selected target IDs;
- typed action, policy decision, before/after state references, result, error, and timing;
- explicit operator takeover/resume boundary where applicable.

The W3C WebDriver BiDi protocol is a useful standards baseline for event-driven browser contexts, script execution, navigation events, network events, and DOM/accessibility locators. Stage should reuse concepts and conformance knowledge where helpful, but must keep Handshake capability, receipt, artifact, and parallel-agent semantics above the renderer protocol.

### Stage Capture records and shared artifact boundary

`StageCaptureCoordinator` accepts a versioned intent containing request/idempotency ID, actor/lease, tab/session IDs, expected tab revision, navigation/document revision, engine generation, source URL/origin/selection, requested part roles, policy references, and optional project/intake destination. An external output path is not part of this command.

The coordinator must:

1. persist intent before renderer activity;
2. validate tab/navigation/document/engine generation before and after collection where staleness matters;
3. request bounded part streams from the adapter;
4. submit streams to the shared ArtifactStore ingest/finalize boundary;
5. record each finalized shared `ArtifactHandle` or explicit partial/failure/reconcile-required state in normalized `StageCapturePart`/source-artifact-link records;
6. submit readability, sanitization, translation, PDF, archive, ASR, export, or intake as separate governed jobs;
7. emit EventLedger/outbox evidence and update compact tab/job projections.

ArtifactStore manifests stay generic. Browser URL/navigation/selection/completeness/engine provenance remains Stage capture lineage. If standalone portable lineage is required, Stage may produce a typed capture-descriptor artifact linked to content artifacts; it must not extend the global manifest into a Stage database.

Current shared storage code is whole-buffer-oriented and must be hardened globally for large Stage outputs: streamed/chunked ingest, incremental hash, bounded buffers/backpressure, atomic idempotent finalize, abort/orphan cleanup, uncertain-finalize reconciliation, range reads, and materialization by handle. Stage cannot create a second store as a workaround.

## Persistence and concurrency proposal

Stage tab/window/session metadata should use the project's canonical PostgreSQL/EventLedger direction after product-authority verification. ArtifactStore owns bytes. Loom owns knowledge-graph entities. No renderer profile database, browser cache, or sidebar projection may become the only source of tab organization.

Proposed mutation path:

1. UI or model submits a typed command with actor, expected revision, and target IDs.
2. Stage validates capability, scope, and canonical entity state.
3. Durable mutation and event/receipt are committed atomically or fail together.
4. UI queries/projectors update from the accepted state.
5. Renderer side effects happen through a supervised command and report success/failure separately.

Navigation and renderer actions cannot always be atomically rolled back with database state. Their receipts must therefore distinguish requested, accepted, dispatched, observed-success, observed-failure, timed-out, and renderer-lost states.

## Lifecycle and resource manager

The lifecycle manager should enforce:

- one machine-wide active/suspended/background-work ceiling followed by fair bounded window/session shares and reserved interactive capacity;
- deterministic eviction ordering informed by visibility, recency, activity, exemption, and cost;
- explicit bounded/expiring exemptions for audio, unsaved work, operator-approved tests, active interactive model work, and separate keep-live holds; pin/bookmark/folder/reminder metadata is never a runtime exemption;
- browser-download protection only until durable job handoff; detached jobs never protect source renderers;
- transition receipts that explain why a tab remained live or was unloaded;
- renderer process/GPU/context cleanup after unload;
- staggered cold restoration and backpressure under memory or CPU pressure;
- no dependency between source-tab liveness and detached workflow jobs;
- no per-record renderer, timer, watcher, poller, subscription, UI/accessibility widget, decoded thumbnail, refresh future, Loom-sync task, or shared-store scan for unloaded/archived records;
- independent attribution/budgets for service workers, browser utilities/GPU processes, workflow jobs, capture/hash/index/thumbnail work, migrations, backup, diagnostics, and ArtifactStore maintenance;
- incremental/paged tree/filter/count/search/accessibility/drag projections rather than hidden full-record work behind visible-row virtualization.

Resource budgets should be operator-configurable but safe by default. Exact numerical targets require profiling on representative hardware before being locked.

## Renderer capability negotiation

Each adapter should report a versioned capability manifest. Stage must fail visibly when a requested operation is unavailable. It must not silently switch engines, silently lose isolation, or quietly produce a weaker artifact.

Capability classes should cover at least:

- navigation/history/input/rendering;
- DOM/script/accessibility observation;
- screenshot/print/capture;
- request/response interception and download handoff;
- cookies/cache/storage/service workers/permissions;
- process isolation/sandbox/crash recovery;
- media codecs/DRM boundary;
- headless/offscreen and visual-debug behavior.

Servo promotion should be based on Stage-level conformance fixtures, not on matching Chromium's internal API.

## Crash, restart, and recovery

- A renderer crash loses only volatile page state; the tab record, organization, jobs, artifacts, and receipts survive.
- A Stage UI crash must not corrupt the canonical tab set or cancel governed background jobs.
- Restart reconstructs windows and records first, then lazily restores selected tabs.
- Partially committed organization mutations must be detected through revisions/events.
- Hung renderers receive bounded timeouts and supervised termination; the user sees a recover/reload/report action.
- Session/profile corruption must be isolated to the affected profile with diagnosis and export/repair options that do not expose secrets.

## Security boundary

Arbitrary web content is hostile. External pages do not receive the privileged Stage App Bridge. Browser/render processes should not share host authority merely because the engine is written in Rust. Multiprocess containment, OS sandboxing, request policy, private-network protection, file access, permission mediation, and crash/hang boundaries remain mandatory research and acceptance gates for Servo.

## Product-code inspection status

The protected product worktree and active WP-1, WP-12, and CKC worktrees were inspected read-only on 2026-07-19. `15-product-topology-and-active-wp-migration.md` records current shared-authority reuse, superseded WP-12 Stage persistence/API/UI, the locked canonical Stage worksurface, migration collisions, and optional real-data import boundaries. This is a dated snapshot rather than an approved implementation baseline. Before final architecture/WP lock, revalidate the exact commits, public schemas, removal/migration sequence, test entrypoints, packaging, and file ownership; do not invent a product structure from this logical design.

## Sources and reuse anchors

- `STAGE-SRC-LOCAL-002`: current Stage behavior and security intent.
- `STAGE-SRC-LOCAL-006`: validated Media Downloader v2 contract.
- `STAGE-SRC-LOCAL-007`: completed Artifact System Foundations contract.
- `STAGE-SRC-LOCAL-008`: Loom graph and PostgreSQL/EventLedger direction.
- `STAGE-SRC-LOCAL-009`: Atelier/Lens intake planning contract.
- `STAGE-SRC-LOCAL-020` through `STAGE-SRC-LOCAL-027`: current protected-product reuse surfaces and active-WP Stage/migration topology.
- `STAGE-SRC-WEB-001` through `STAGE-SRC-WEB-006`: current Servo API, architecture, release, and source evidence.
- `STAGE-SRC-WEB-016`: W3C WebDriver BiDi specification.
- `STAGE-SRC-WEB-017`: AccessKit architecture for native accessibility projection.

</topic>
