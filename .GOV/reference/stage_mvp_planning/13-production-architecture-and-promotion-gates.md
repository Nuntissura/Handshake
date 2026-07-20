---
file_id: stage-production-architecture-and-promotion-gates
file_kind: reference-technical-design
updated_at: "2026-07-19"
status: proposed-for-operator-review
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-production-architecture" status="proposed-for-operator-review" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage production architecture

## Purpose and authority

This document turns the current Stage research and local-source reconciliation into a proposed production design. It is nonauthoritative. It does not modify the Master Spec, activate the Stage WP, or supersede any packet.

## Product boundary

Stage owns native browser workspace behavior: durable windows/sessions/tabs, engine adapters, rendering attachment, browser observation and input, request policy, operator takeover, capture/import initiation, high-volume tab organization, and Stage-specific diagnostics. The Stage-facing capture process is Stage Capture, implemented at the planning boundary by `StageCaptureCoordinator`.

Stage does not own:

- general model/provider orchestration, ToolGate, process supervision, promotion, Flight Recorder, EventLedger, or Argus; those remain WP-1/Dexterity responsibilities;
- canonical artifact bytes, manifests, hashes, retention, materialization, or garbage collection; those remain shared Artifact System responsibilities even when `StageCaptureCoordinator` initiated the work;
- media download implementation; that remains Media Downloader;
- ASR execution; that remains the ASR work packet;
- editor document authority or editor embed-back; those remain WP-12;
- CKC pose/character data authority; CKC consumes governed Stage artifacts and lineage;
- durable knowledge-graph authority; Loom owns LoomBlocks, tags, mentions, backlinks, pins, and graph relations.

## Runtime composition

```text
Native Stage shell
  -> canonical Stage registry and state machine
  -> command/event and receipt service
  -> host request-policy pipeline
  -> browser-engine adapter
       -> WebView2 bootstrap (Windows interactive)
       -> Chrome-for-Testing worker (optional headless validation)
       -> Servo adapter (strategic; restricted alpha until security gates pass)
       -> CEF fallback (only after explicit escalation gate)
  -> Workflow Engine / WP-1 services
  -> StageCaptureCoordinator
       -> shared ArtifactStore / Downloader / ASR / Loom / Lens / Atelier contracts
  -> Flight Recorder / EventLedger / Argus projections
```

The native shell owns layout, input routing, visible focus, accessibility integration, virtualized sidebars, overlays, diagnostics, and operator controls. Each browser adapter has a single runtime owner and communicates through bounded queues; no adapter handle is passed to model, editor, CKC, or background-worker code.

## Canonical records

The later spec should define structured, revisioned records rather than string handles or engine-native objects:

| Record | Required identity and role |
|---|---|
| `StageWindow` | Stable window identity, project/workspace association, active tab, sidebar view state, revision. |
| `StageSession` | Browser credential/storage policy, engine preference, persistence/retention class, trust class, revision. It is distinct from WP-1 `ModelSession`. |
| `StageTab` | Stable tab identity, window/session, URL/title, organization state, lifecycle facets, active engine attachment, revision. |
| `BrowserAttachment` | Adapter and capability version, engine generation, ephemeral context/target/process IDs, profile materialization, health. |
| `StageActionIntent` | Durable pre-side-effect command, expected revision, target, policy decision, idempotency/correlation data. |
| `StageActionReceipt` | Dispatch, result or uncertain outcome, observations, postcondition, replay/reconciliation state. |
| `StageCapture` | Stage-owned capture/lineage record: capture kind, source tab/navigation/generation, provenance, shared opaque artifact handles, extraction/sanitization policy, job/correlation state, and downstream disposition. It is not an artifact manifest or byte store. |
| `StageBulkRun` | Frozen canonical target snapshot, counts, cursor, bounded concurrency, per-target results, reconciliation. |

All cross-subsystem artifacts use the structured canonical shared `ArtifactHandle` selected during future baseline binding. Current Master Spec and code contain differing handle representations; Stage must import the selected shared type as opaque and cannot introduce another URI, string-only wire type, or Stage-specific encoding. Database tables and events carry schema versions, stable IDs, optimistic revisions, and correlation IDs. Engine IDs are never durable authority.

## Lifecycle model

Lifecycle is expressed as independent facets:

- record: `OPEN`, `CLOSED`, `ARCHIVED`;
- runtime attachment: `DETACHED_COLD`, `STARTING`, `ATTACHED`, `CRASHED`, `STOPPING`;
- page: `ACTIVE`, `PASSIVE`, `HIDDEN`, `FROZEN`, `DISCARDED`, `TERMINATED`, `UNKNOWN`;
- navigation: `IDLE`, `STARTED`, `COMMITTED`, `INTERACTIVE`, `COMPLETE`, `FAILED`;
- control: `UNLEASED`, `MODEL_LEASED`, `OPERATOR_LEASED`, `BLOCKED_OPERATOR`, `CANCELLING`.

The canonical registry persists 3,000-plus records while the resource scheduler maintains a bounded live set. Restore is lazy and prioritized. Unsaved input, active media/WebRTC, explicit expiring keep-live holds, operator holds, active test runs, and active model leases are scheduler protections with visible owner/reason/expiry. A browser download is protected only until durable job handoff; the detached job cannot keep the renderer alive. Pinned, bookmarked, foldered, labeled, reminder, and active-window records are metadata and never implicit renderer protections. The UI projects the registry; it never becomes the data source for bulk action or count proof.

## Resource, projection, and background-work contract

- Machine-wide live and suspended renderer ceilings are authoritative; session/window budgets are fair bounded shares and cannot multiply the global ceiling.
- Every admission, protection, eviction, and failed eviction exposes deterministic `why_awake`/budget evidence.
- Exemptions have ceilings, expiry/renewal, fairness, and an explicit over-budget disposition; dirty state cannot be silently discarded.
- Dormant records create no per-record renderer, timer, watcher, polling loop, database subscription, UI/accessibility widget, decoded thumbnail, refresh job, Loom-sync future, or maintenance task.
- Sidebar/tree flattening, filtering, counts, accessibility projection, drag/drop, and search use incremental invalidation or bounded paging; visible-row painting cannot conceal an O(n) per-frame or per-event walk.
- Service workers, push/background sync, browser utilities, GPU processes, workflow workers, profile cleanup, migration/reindex, thumbnail work, backup, capture, ArtifactStore retention/health, and diagnostics are independently attributed and budgeted.
- Shared ArtifactStore health/retention scans cannot run per tab, per sidebar refresh, or as an unbounded Stage-startup dependency. `StageCaptureCoordinator` submits bounded work and receives finalized handles or an explicit partial/failure result; it never scans or maintains the store.
- Stage never periodically wakes every dormant tab for freshness. Page freshness is restored on selection or through an explicit bounded job.
- Resource proof uses 1/10/100/1,000/3,000/10,000-record scale curves, a mandatory realistic 3,000-plus fixture, and simultaneous multi-window/session runs.

## Browser adapter contract

Each adapter publishes its name, build/version, protocol versions, security mode, profile model, supported platforms, and capability manifest. The invariant Stage surface covers:

- create/attach/close background or visible contexts;
- navigation and lifecycle events;
- focus policy and input actions;
- semantic snapshot, geometry/actionability, screenshot, and script evaluation;
- console/network/download observation;
- request inspect/block/redirect/fulfil;
- profiles, cookies, storage clearing, and ephemeral cleanup;
- freeze/discard/restore where supported;
- resource and process metrics;
- process failure, restart, and graceful/hard termination.

Adapter-specific enhancements remain optional and explicit. Unsupported behavior returns `UNSUPPORTED_CAPABILITY`; fallback requires an operator-visible decision and a new receipt.

## Engine rollout

### Chromium bootstrap

- Direct WebView2 is the proposed Windows interactive bootstrap.
- Child-HWND integration is tested first; CompositionController is a bounded fallback for composition defects, not offscreen rendering.
- Chrome-for-Testing is an optional isolated worker for deterministic headless validation.
- CEF requires an explicit escalation after WebView2 prototype evidence.
- Chromium-specific product logic freezes at the adapter boundary; the shared Stage domain/native shell may implement the complete `STAGE-WIN-BOOTSTRAP-PROD` slice through WebView2 capabilities without turning Chromium into a coequal strategic backend.
- WebView2 Google authentication is a confirmed compatibility blocker for the primary authenticated YouTube journey. Windows production promotion requires the explicit authentication/session-acquisition contract and journey evidence in `16-web-compatibility-auth-and-agent-security.md`.

### Servo strategic adapter

- Pin exact release tag, commit, Cargo lockfile, Rust toolchain, features, generated API docs, and SBOM.
- Run one Servo owner event loop and marshal actions through queues because `WebView` is not `Send` or `Sync`.
- Separate Servo instances/processes across hard trust boundaries until isolation is positively proven.
- Stage owns Windows WPT, journey, security, crash, soak, packaging, and upgrade qualification.
- Trusted/allowlisted-content alpha is eligible only after embedding, isolation, storage-cleanup, compatibility, recovery, and operational gates pass.
- Arbitrary-web Windows use is blocked until effective default-deny content-process sandboxing exists and independently passes escape-oriented tests. Multiprocess alone is insufficient.

Servo remains the strategic direction, but "default renderer" must be capability- and platform-qualified. The spec must not promise insecure arbitrary-web behavior to preserve a calendar target.

### Release and WP closure separation

- `STAGE-WIN-BOOTSTRAP-PROD` may release a complete production-qualified WebView2 Windows slice after every applicable product, security, recovery, packaging, backup, accessibility, manual, and support gate passes.
- `STAGE-SERVO-RESTRICTED-ALPHA` is a separate visibly restricted Servo milestone.
- `STAGE-SERVO-ARBITRARY-WEB` remains security-blocked on Windows until effective content-process sandbox proof exists.
- `STAGE-WP-COMPLETE` remains strategic Servo and full-scope completion. A Windows product release cannot falsely close the open Servo work.

## Security and trust zones

1. External web content is hostile and has no privileged Stage bridge.
2. Stage Apps use a separate trusted origin/package class, versioned bridge schema, explicit method/capability IDs, Workflow Engine/ToolGate checks, and Flight Recorder/EventLedger receipts.
3. Captured HTML remains hostile unless transformed into an explicitly sanitized artifact with policy/version provenance, script removal, constrained resource resolution, and bridge-denial proof.
4. File, loopback, private-network, custom-scheme, redirect, download, popup, permission, authentication, and certificate behavior use a host-owned versioned request policy.
5. Browser automation endpoints bind only to private pipes or randomized loopback endpoints, use non-default isolated profiles, rotate capabilities, and never disable the browser sandbox.
6. Profile and injected-content sharing across trust boundaries is forbidden until isolation tests prove it safe.
7. A CAPTCHA or authentication interruption becomes `BLOCKED_OPERATOR`; operator takeover is a recorded lease transfer.

## Agent control and WP-1 integration

Stage's command protocol is browser-engine-neutral and WebDriver-BiDi-shaped. A command references stable Stage IDs plus expected state revision and ephemeral engine generation. Before dispatch, a durable intent is written with WP-1 lane/run, ToolGate/consent result, idempotency, causation, correlation, trace, deadline, focus policy, and pre-observation hash.

On timeout or crash, an uncertain side effect becomes `OUTCOME_UNKNOWN_RECONCILE_REQUIRED`. Stage observes and reconciles before retrying non-idempotent operations. Observation combines semantic nodes, element references, geometry, viewport/actionability, screenshot, URL/navigation, lifecycle, focus, resources, console/network facts, and postconditions.

WP-1 integration cannot be marked production-ready until its current process-ownership, terminate/reap, stop-result, stale-diagnostic, capability-projection, and claim-generation blockers are closed. Stage should define contract tests now and consume the proven interfaces later.

External web text is untrusted data, never authority. Before model dispatch Stage applies source-to-sink, consequence, capability, lease/fencing, revision/generation, confirmation/watch/takeover, and sensitive-data policy. URL navigation, forms, uploads, cross-origin requests, and tool calls are data sinks. Prompt-injection detection is one defense layer; production proof requires no unauthorized action or data exfiltration in the adversarial corpus.

## WP-12 Stage supersession and CKC interaction

WP-12's active Stage prototype is inspected evidence, but the operator has superseded all of its Stage-specific UI, API, storage, adapter, connector, route, schema, and mockup authority. Before Stage implementation:

- decide from current requirements whether editor-to-Stage and embed-back workflows exist, then specify them anew without WP-12 aliases;
- remove/replace `StagePane`, `ModuleId::Stage` placeholder behavior, `/stage/artifacts`, `StageArtifactRefWire`, `stage_capture_artifacts`, `stage.jobs.enqueue`, and other old Stage surfaces;
- inventory real operator data separately from test/mock data and perform a verified one-way ArtifactStore/current-Stage import only where real data requires retention;
- re-prove current hash, provenance, idempotency, stale-target, and correlation outcomes rather than inheriting the old wire protocol;
- coordinate collision-free database removal/replacement and optional import before either branch lands.

CKC remains a downstream consumer. Stage supplies canonical artifact handles, capture/source lineage, media/3D facts, and correlation IDs; CKC supplies pose/character/project domain semantics. No CKC implementation is folded into the Stage WP.

## Capture, artifacts, Downloader, and ASR

- Current-renderer capture records the active authenticated/script-mutated renderer state.
- Independent acquisition/archive is a separate job and should use an archival format such as WARC where replay-grade capture is intended.
- Selection/media extraction produces canonical artifacts and lineage for Downloader, ASR, Loom, Lens, Atelier, CKC, and archive consumers.
- Downloads continue independently after a source tab unloads.
- Routine authenticated downloads use a governed scoped session-credential lease/adapter. Explicit cookie JSON export remains available but is redacted, scoped, encrypted when persisted, and never logged.
- The folded media-portability and ASR-lineage requirements remain full Stage-WP lanes with independent acceptance criteria, external dependencies, and downstream contracts.

## Operator and model usability

The native shell exposes stable action IDs, keyboard and command-palette routes, visible engine/security/session state, per-window virtualized sidebar, folders/groups/color labels/bookmarks, Loom relations, capture/download/translation/export actions, model-control indicators, operator takeover, and recovery controls.

The shipped UserManual must let an operator or no-context model:

- create and isolate a session;
- operate and organize a high-volume tab set;
- inspect engine/capability/security state;
- capture, download, ingest, and trace an artifact;
- invoke and revoke model control;
- recover from discarded tabs and crashed engines;
- diagnose a blocked request or failed action from receipts;
- find validation evidence without prior chat context.

## Diagnostics and validation surface

Every Stage run exposes structured state, logs, traces, health checks, error reports, screenshots, process/profile mapping, adapter/runtime versions, capability manifests, request-policy decisions, lifecycle transitions, command receipts, and reproducible journey IDs. Argus-compatible visual inspection checks readability, discoverability, navigation, overlap, responsive layout, visible important state, and no focus theft.

The validation corpus includes controlled fixtures plus representative authentication, forms, uploads, downloads, media, modern JavaScript, accessibility, storage, PDF, 3D, capture, translation, search, hostile HTML, private network, and crash/restart workflows. WPT/standard suites support but do not replace Stage-owned journeys.

## Production gates

| Gate | Minimum proof |
|---|---|
| Adapter conformance | Fake adapter plus each real adapter passes invariant command/event, unsupported-capability, stale-generation, and no-silent-fallback tests. |
| Native integration | DPI, multi-monitor, focus, IME, accessibility, clipping, overlays, popups, downloads, context menus, device loss, and quiet background operation pass. |
| Isolation/security | No cross-profile leakage; bridge inaccessible to hostile web; private/file/custom schemes enforced; browser sandbox enabled; automation endpoints private; crash cleanup proven. |
| High-volume state | 3,000-plus durable records, bounded live set, lazy restore, virtualized UI, canonical-set bulk actions, exact reconciliation, measured CPU/memory/disk/latency. |
| Agent control | Durable pre-dispatch intents, complete receipts, operator takeover, uncertain-outcome reconciliation, stale-target rejection, no focus theft, WP-1 correlations. |
| Capture/lineage | Renderer versus archive semantics distinguished; canonical ArtifactHandle; hashes/provenance; downstream Downloader/ASR/Loom/Lens/Atelier/CKC contracts proven. |
| Failure recovery | Freeze, discard, renderer/GPU/browser crash, host hard-kill, event loss, network failure, restart, orphan cleanup, bounded retry, rollback. |
| Supply chain | Exact pins, reproducible build, SBOM, notices, advisory scan, runtime update qualification, state-preserving restart, rollback. |
| Usability/manual | Operator and no-context-model journeys pass using shipped UI/manual only; diagnostic evidence is discoverable. |
| Servo restricted alpha | Servo-specific embedding, isolation, WPT subset, journey suite, storage cleanup, soak, crash, packaging, and rollback evidence. |
| Servo arbitrary-web Windows | Effective content-process sandbox plus independent negative/escape-oriented validation. This gate is currently BLOCKED by upstream capability. |

No gate is satisfied by a planning document. Promotion requires runtime evidence from the target branch/build.

</topic>

<topic id="stage-production-red-team" status="proposed-for-operator-review" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Production red team

| Failure scenario | Impact | Minimum control and proof |
|---|---|---|
| Chromium bootstrap quietly becomes permanent and feature-complete. | Duplicate engines and retirement failure. | Adapter-only domain logic, feature freeze, source audit for leaked vendor types, explicit retirement gate. |
| Servo multiprocess is mistaken for sandboxing. | Host compromise from hostile content. | Windows arbitrary-web hard block until effective sandbox and negative tests pass. |
| Stage, WP-12, and WP-1 each define their own Stage job/capability/session types. | Semantic drift and unsafe orchestration. | Supersede/remove WP-12 Stage types, consume current WP-1 shared authorities, create only current Stage contracts, and prove no duplicate persistence authority remains. |
| A model action times out after the page changed. | Duplicate purchase/post/delete or other side effect. | Durable pre-dispatch receipt, unknown-outcome state, observation/reconciliation before retry. |
| Bulk close/archive touches only visible rows. | Incomplete operation and false counts. | Frozen canonical target artifact and exact reconciliation against the registry. |
| A discarded tab loses unsaved state or active work. | Operator data loss. | Protection reasons, pre-discard checkpoint where supported, explicit warning, recovery test. |
| Profile isolation leaks cookies or service-worker state. | Cross-project/account contamination. | Storage-surface isolation matrix across live, restart, crash, and cleanup paths. |
| External web obtains the trusted Stage bridge. | Privilege escalation. | Origin/package binding, capability/method separation, hostile-content bridge-negative harness. |
| Browser profile files are treated as durable authority. | Corruption, version lock, nonportable state. | Canonical database records, materialization/rebuild path, versioned explicit export/import only. |
| Capture silently refetches instead of recording the active logged-in page. | Wrong evidence and lost authenticated state. | Separate capture kinds and UI labels; navigation/generation provenance and comparison tests. |
| Stage ships without process/recovery ownership from active WP-1. | Orphan browsers and false stop status. | Block dependent production gate until WP-1 ownership/reap/stop proof is green. |
| Migration numbers collide across active worktrees. | Integration failure or reordered schema history. | Reserve/rebase migration sequence immediately before merge and test upgrade from the common base. |
| Diagnostics contain raw cookies, tokens, or captured secrets. | Credential disclosure. | Structured redaction at source, canary-secret tests, export confirmation, no raw values in logs/receipts. |

</topic>
