---
file_id: stage-mvp-native-frontend-and-tab-workspace
file_kind: reference-architecture-plan
updated_at: "2026-07-19"
status: proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-native-frontend-and-tab-workspace" status="proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Native Stage frontend and tab workspace

## Planning outcome

Stage should be a native Handshake workspace whose tab, folder, bookmark, label, project, and job state remains useful when no web renderer exists. A live webview is a temporary execution resource attached to a durable tab record, not the tab's identity or source of truth.

This makes the operator's 3,000+ tab workload a metadata-and-index problem for most of the session. CPU, memory, process, and GPU use should be driven primarily by a small configured live-renderer budget rather than by total tab count.

All component names and layouts in this note are planning proposals. They are not product authority until the operator approves them and they are propagated through the master spec and work-packet workflow.

## Proposed operator shell

### Global Stage bar

The top bar should expose:

- back, forward, reload/stop, and address/search controls;
- current Stage Session and its isolation state;
- current Handshake project and intended intake consumer;
- renderer badge (`Servo` or bootstrap `Chromium`) and any capability limitation;
- primary actions for capture, download, translate, export, inspect, and model control;
- visible privacy, network-policy, and agent-activity indicators.

The renderer badge is evidence, not a backend selector for routine use. Normal product direction is Servo. Chromium is a minimum bootstrap/compatibility path and must not silently take over when Servo lacks a capability.

### Per-window tab sidebar

The left sidebar is the primary high-volume control surface. It should support:

- nested folders/groups within the current Stage window;
- virtualized tab rows so only visible rows allocate UI widgets;
- collapse/expand, multi-select, range-select, and select-all over the canonical result set;
- color labels, bookmark state, project association, reminder note, and Loom-link indicators;
- lifecycle badges for active, suspended, unloaded, archived, audio, pending-download-handoff, detached-job, unsaved, test-run, and keep-live states;
- inline fuzzy search plus structured filters;
- saved views such as unwatched, not-ingested, duplicate URL, downloaded, project-linked, recently opened, and keep-live;
- drag/drop and keyboard reorganization without instantiating page renderers;
- bulk conversion of selected URLs into one existing Media Downloader batch;
- bulk archive/unload/label/bookmark/project-link actions that operate on canonical IDs, not rendered rows.

The UI should flatten the visible portion of the expanded folder tree into a virtual row model. Folder counts, selection counts, and bulk actions must be calculated from the canonical tab query, not from the currently painted range. Flattening, filtering, counts, search projections, accessibility nodes, and drag/drop targets must use incremental invalidation or bounded paged queries; row virtualization is not sufficient if every event still performs a full-tree walk or full-result recount.

The egui `ScrollArea::show_rows` API is a relevant implementation pattern because it only materializes the visible range for large fixed-height row sets. The exact component must be proved against nested folders, variable-height notes, keyboard navigation, accessibility, and drag/drop before being locked.

### Content viewport

The center viewport hosts the selected live webview. Selecting an unloaded tab should:

1. allocate a renderer slot or ask which visible exemption may be displaced;
2. bind the tab's Stage Session and policy context;
3. create the selected renderer adapter explicitly;
4. navigate to the saved restoration target;
5. show restore progress and explain any lost volatile state;
6. attach new observations and history to the durable tab record.

Stage should never restore every historical tab merely because a window was opened. It should restore the window/folder/tab records immediately and instantiate only the selected tab plus the configured warm set.

### Context and evidence drawer

A collapsible right drawer should expose context without crowding the web page:

- tab metadata, normalized URL, opened-from/watch-after relationships, notes, and project links;
- current session, permissions, storage usage, cookie summary, and keep-live reason;
- capture/download/translation/export jobs and resulting ArtifactHandles;
- LoomBlock, tag, mention, backlink, and collection relationships;
- model observations, pending approvals, action receipts, and replay/debug evidence;
- console, navigation, network-policy, and renderer health summaries.

Raw secrets, cookie values, and unbounded page payloads must not appear in this drawer or its receipts.

### Job and health shelf

A compact bottom shelf should keep background operations visible after their source tabs unload. It should show bounded progress for Media Downloader, capture, ASR, translation, indexing, export, and project intake jobs. Closing a tab must not cancel its detached job. Cancellation must target the job explicitly.

## Tab lifecycle presented to the operator

| State | Renderer | Background page activity | Durable tab record | Restore behavior |
|---|---:|---:|---:|---|
| `ACTIVE` | live | allowed by policy | yes | already interactive |
| `SUSPENDED` | live but throttled | timers/media/network constrained by policy | yes | fast resume where supported |
| `UNLOADED` | none | none | yes | create renderer and navigate lazily |
| `ARCHIVED` | none | none | yes, outside active window | return record to a window or open directly |

The state names are proposed. The behavioral invariants matter more than their labels:

- unloaded and archived tabs cannot run page code, media, timers, service-worker activity on behalf of that tab, or content indexing;
- cookies and site storage belong to the Stage Session and are not erased merely because one tab unloads;
- HTTP/media cache is bounded separately by quota and eviction policy;
- active downloads are workflow jobs, not browser-tab activity;
- a browser download may hold a renderer only until durable job handoff succeeds; handoff failure is visible and bounded rather than an indefinite exemption;
- pin, bookmark, folder, label, reminder, and active-window membership are organization metadata, not renderer protections;
- `keep_live` is a separate explicit hold with owner, reason, start time, expiry/renewal, priority, and budget cost;
- visible exemptions must explain why a renderer remains live.

## Resource scheduler and dormant-record contract

Stage must preserve very large durable tab collections without creating a proportional amount of hidden runtime work:

- live renderer and suspended-renderer ceilings are global first, with bounded session/window shares and reserved interactive capacity; opening more windows cannot multiply the machine-wide ceiling;
- every renderer admission, protection, eviction, and failed eviction has a deterministic reason visible in a `why_awake`/resource diagnostic;
- exemptions have hard ceilings, fairness, expiry, renewal rules, and an operator-visible disposition when protected work exceeds the budget;
- an unloaded or archived tab allocates no renderer, UI widget, decoded thumbnail, timer, watcher, polling loop, database subscription, network refresher, favicon/title refresher, Loom-sync task, or per-record maintenance future;
- metadata refresh, thumbnail decode, search/index maintenance, Loom projection, migration, backup, capture, ArtifactStore maintenance, and diagnostics use shared event-driven queues with bounded concurrency, rate limits, cancellation, backpressure, and caches;
- profile-level service workers, push/background sync, browser utilities, GPU processes, download workers, and extension-like services are independently attributed and budgeted; they cannot be mislabeled as zero activity merely because a tab renderer is detached;
- suspended WebView2 is a best-effort intermediate state, not proof of quiescence; older or over-budget pages are fully detached/destroyed according to policy;
- no background policy periodically wakes every suspended or unloaded tab to refresh it; freshness is obtained on selection or through an explicit bounded job;
- ordinary startup restores canonical records and projections before any page, and separately schedules only the selected tab plus the admitted warm set;
- ArtifactStore and other shared-subsystem scans never run per tab or per sidebar refresh and remain outside the hot tab-state path.

The performance contract is evaluated at 1, 10, 100, 1,000, 3,000, and stretch 10,000 records so accidental linear per-record idle work is visible. The 3,000-plus operator fixture remains the mandatory acceptance case.

## Reminder and watch-queue behavior

The operator uses tabs as ordered reminders. Stage should preserve that intent directly rather than forcing every reminder to masquerade as a live page:

- `opened_from_tab_id` preserves discovery lineage;
- a watch-after relationship or ordered folder view preserves intended sequence;
- `pending`, `watched`, `downloaded`, `ingested`, `skipped`, and `needs-review` are explicit workflow states;
- thumbnails and titles may be refreshed only during normal navigation or an explicit bounded metadata job;
- duplicate URL detection groups records but never deletes distinct notes, folder membership, ordering, or operator intent automatically;
- playlist/channel conversion offers a downloader batch while retaining the original reminder records and provenance.

## Frontend-to-backend action map

| Frontend action | Backend owner | Durable result |
|---|---|---|
| Open/switch tab | lifecycle manager plus renderer supervisor | renderer binding and navigation receipt |
| Move/label/bookmark tabs | tab catalog | event-backed metadata update |
| Search tabs | search/query service | canonical result IDs and rank evidence |
| Download selection/page/media | Stage intake orchestration plus existing Media Downloader v2 | workflow job ID, artifact results, tab/job links |
| Capture page/selection | `StageCaptureCoordinator` plus shared ArtifactStore | StageCapture lineage, shared ArtifactHandle, provenance, optional LoomBlock |
| Translate page/selection | translation coordinator | derived artifact linked to original |
| Export Markdown/PDF | export pipeline | ExportRecord and materialized paths |
| Let a model interact | agent-control gateway | capability decision, observation/action receipts, screenshots/traces |
| Edit/export cookies | session manager plus secret/export gates | session mutation or governed secret export record |

## Accessibility and model usability

The native chrome and embedded content need stable accessibility identities and model-facing action identifiers. The model path should prefer structured DOM/accessibility targets and use visual screenshot/OCR/pointer control when structured targets do not exist or do not represent canvas-heavy UI. Both paths must re-observe after navigation or mutation and reject stale targets.

Model actions must be quiet by default: no foreground window creation, focus stealing, or keyboard hijacking. If an operator takeover is required for a CAPTCHA, permission, or ambiguous high-impact action, Stage should pause the model action, make the blocking state visible, and resume from a fresh observation after takeover.

## UI proof obligations

- Load a realistic 3,000+ tab-record window with deep folders, labels, bookmarks, notes, and mixed lifecycle states.
- Prove that only visible sidebar rows are materialized and that scroll/search/bulk selection remain coherent.
- Prove that select-all and bulk actions cover the canonical filtered set, including rows never painted.
- Prove that window restore does not instantiate all renderers.
- Prove that an unloaded tab generates no page activity.
- Prove that unloaded-record count does not create per-record timers, tasks, watchers, subscriptions, UI allocations, decoded thumbnails, network refreshes, or shared-store scans.
- Prove that pin/bookmark/folder metadata does not consume renderer budget and that explicit keep-live holds remain bounded, expiring, attributable, and diagnosable.
- Prove global renderer/background-work ceilings and fairness across several simultaneous windows and sessions rather than only one large window.
- Prove incremental folder/search/count/accessibility projections and bounded allocation during mutation, filtering, dragging, expanding, and bulk actions; visible-row rendering alone is insufficient.
- Prove that downloads and intake jobs remain visible and continue after source-tab unload or close.
- Prove keyboard navigation, accessibility names, focus order, text clipping, responsive layout, and high-DPI behavior through the required visual-debugging surface.
- Record renderer, live-budget, session, job, and agent activity in a no-context-model-readable diagnostic view.

## Sources and reuse anchors

- `STAGE-SRC-WEB-001`: Servo WebView lifecycle and throttling API.
- `STAGE-SRC-WEB-010` through `STAGE-SRC-WEB-014`: field patterns for throttling, discarding, hibernation, and lazy restore.
- `STAGE-SRC-WEB-015`: egui visible-row rendering for large row sets.
- `STAGE-SRC-WEB-090`: WebView2 suspend semantics and best-effort caveats.
- `STAGE-SRC-WEB-091`: WebView2 process inventory, attribution, and failure diagnostics.
- `STAGE-SRC-WEB-092`: Chrome Page Lifecycle freeze/discard semantics and volatile-state caveats.
- `STAGE-SRC-WEB-093`: egui large-scroll-area CPU warning and visible-only layout guidance.
- `STAGE-SRC-LOCAL-002`: current Stage cold-tab, session, capture, and model-action intent.
- `STAGE-SRC-LOCAL-006`: existing Media Downloader v2 contract and job schemas.
- `STAGE-SRC-LOCAL-008`: LoomBlock/tag/mention/backlink graph authority.

</topic>
