---
file_id: stage-rw-011-high-volume-tab-lifecycle
file_kind: reference-research-note
updated_at: "2026-07-20"
research_workstream: STAGE-RW-011
verification_status: hardened-primary-sources-and-current-code-checked
---

<topic id="stage-rw-011-high-volume-tab-lifecycle" status="hardened" version="v0.3" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-20">

# High-volume tab lifecycle research

## Question

Should Stage clear cache aggressively to support 3,000+ tab reminder/watch queues with low CPU and memory use?

## Finding

No. The primary control should be page freezing/throttling followed by renderer discard/unload. HTTP cache eviction is a separate disk-quota concern.

Chromium documents that dropping graphics caches only produces short-lived memory relief, while discarding a tab terminates its renderer state and produces a persistent memory reduction. Current Chromium Memory Saver and WebExtensions tab-discard behavior keep the tab record visible and reload it when reactivated. Microsoft Edge distinguishes sleeping tabs, which freeze scripts to minimize CPU, from discarded tabs, which release the page's CPU and memory at the cost of full reload.

Servo's current embedding API provides `WebView::set_throttled`; its WebView documentation also states that dropping the last WebView handle closes the webview and cleans up its associated resources. This supports a host-owned Stage lifecycle with suspended and fully unloaded states.

WebView2 `TrySuspendAsync` confirms that suspend is best effort: the WebView must be invisible, running script may delay suspension, and later API calls can auto-resume it. Suspension is therefore an intermediate optimization, not proof that a dormant tab has no CPU or network cost. WebView2 process APIs and failure events allow browser/renderer/GPU/utility attribution, but Stage must deduplicate shared-process events and separately account for activity not represented by a tab renderer.

Chrome Page Lifecycle guidance confirms that discard is not reliably observable at discard time and termination callbacks cannot be trusted to preserve last-second state. Stage must preserve known durable state before eviction, classify volatile state honestly, and use restoration evidence rather than claiming perfect live-page continuity. Egui's own CPU guidance confirms that visible-row APIs are necessary but not sufficient: the application must also avoid full collection layout/recount work each frame.

## Recommended Stage pattern

1. Persist tab identity and organization independently from renderer identity.
2. Keep only a configured active working set instantiated.
3. Throttle recent inactive webviews only inside a small suspended budget and verify actual suspension/process activity.
4. Destroy older or over-budget inactive webviews while keeping their durable tab records.
5. Restore an unloaded tab by creating a new webview and navigating to the saved URL/history contract.
6. Preserve cookies/site storage at the session level unless the user explicitly clears them.
7. Bound HTTP/media cache by disk quota and LRU policy; do not clear it on each unload.
8. Exempt visible tabs, audio playback, unsaved forms, active capture/test runs, and explicit keep-live tabs according to a visible bounded/expiring policy. Pin/bookmark/folder state is not a renderer exemption. Active downloads are the one unconditional exemption (`STAGE-DEC-019`): a renderer carrying a not-yet-handed-off download is never suspended, throttled, or unloaded until the download completes or durable job handoff succeeds; the exemption is visible in `why_awake` but never expires while the download is active. Detached jobs do not keep source tabs live.
9. Apply one machine-wide renderer/background-work ceiling before fair session/window shares so opening additional windows cannot multiply resource budgets.
10. Allocate no per-record timers, watchers, polling loops, subscriptions, UI/accessibility nodes, decoded thumbnails, refresh futures, or shared-store scans for unloaded/archived records.
11. Treat profile service workers, push/background sync, utility/GPU processes, workflow jobs, thumbnail/index maintenance, migrations, backup, diagnostics, and ArtifactStore maintenance as separate attributed/budgeted workloads.
12. Use incremental/paged folder, filter, count, search, accessibility, and drag/drop projections; virtualized painting must not hide a full-record walk.

## Rejected or constrained options

- Aggressive HTTP-cache clearing as the primary control: it does not stop live execution and increases later reload/network work.
- Keeping every pinned/bookmarked/reminder tab warm: it collapses the distinction between organization and runtime and defeats a hard renderer ceiling.
- Periodically waking all suspended tabs for freshness: it creates CPU/network storms proportional to tab count.
- Per-tab polling for title/favicon/Loom/search state: it creates hidden host CPU even when renderer count is bounded.
- One budget per window without a global ceiling: multiple windows multiply machine load.
- Row virtualization alone: it does not prevent full-tree flattening, recount, filtering, accessibility projection, or thumbnail decoding.
- ArtifactStore validation/health scans during Stage startup or sidebar events: the shared store is not part of the hot tab-state path.

## Failure scenarios and mitigations

- Unsaved form state lost on unload: detect or conservatively exempt pages with active form/test state; show unload eligibility.
- Audio or download interrupted: audio receives a bounded visible activity exemption. Downloads are never interrupted by lifecycle machinery (`STAGE-DEC-019`): a renderer-bound download unconditionally protects its carrier renderer until completion or durable independent-job handoff succeeds, after which the source renderer is unloadable; only explicit operator/job cancellation or genuine external failure terminates a download.
- Restoration causes network spikes: stagger restores, keep bounded cache, instantiate only selected tabs.
- Thousands of metadata rows strain the UI: virtualized sidebar and indexed search over the canonical tab store.
- Background indexing wakes every page: index only metadata already present and content captured during normal use or explicit jobs.
- Thousands of pinned tabs stay live: make pin metadata-only and require a separate budgeted/expiring keep-live hold.
- Renderer count is bounded but host CPU still scales: prohibit per-record timers/tasks/subscriptions/widgets and measure the host-work scale curve.
- Service workers or utility processes keep profiles active: attribute by profile/process/frame, apply separate budgets, and expose quarantine/clear controls.
- Protected states saturate the machine: apply ceiling, fairness, expiry/renewal, owner/reason diagnostics, and an explicit over-budget disposition.
- Multi-window use multiplies caps: enforce a machine-wide ceiling with reserved interactive capacity and fair shares.
- Shared maintenance creates disk/CPU spikes: make migration/reindex/thumbnail/backup/ArtifactStore health work incremental, rate-limited, pausable, and independent from tab/UI events.

## Sources checked

- `STAGE-SRC-WEB-001`: Servo WebView API.
- `STAGE-SRC-WEB-010`: Chromium tab discarding and reloading design.
- `STAGE-SRC-WEB-011`: Chrome Memory Saver behavior.
- `STAGE-SRC-WEB-012`: Microsoft Edge sleeping versus discarded tabs.
- `STAGE-SRC-WEB-013`: MDN `tabs.discard()` behavior.
- `STAGE-SRC-WEB-014`: Vivaldi hibernation and lazy session loading.
- `STAGE-SRC-WEB-015`: egui visible-row rendering.
- `STAGE-SRC-WEB-090`: WebView2 suspend semantics and caveats.
- `STAGE-SRC-WEB-091`: WebView2 process and failure attribution.
- `STAGE-SRC-WEB-092`: Chrome Page Lifecycle freeze/discard behavior.
- `STAGE-SRC-WEB-093`: egui CPU guidance for large collections.

## Validation plan

- Run a common protocol at 1, 10, 100, 1,000, 3,000, and stretch 10,000 records with a defined warm-up, settle, idle sample, interaction, and recovery phase.
- Benchmark the mandatory realistic 3,000-plus fixture with only the selected and configured bounded warm set live, then repeat across simultaneous windows/sessions.
- Measure host and per-process CPU, resident memory, GPU allocation, renderer/utility process count, network/disk activity, task/timer/subscription counts, decoded-thumbnail cache, queue depths, and UI/query/allocation latency.
- Verify no network request, page timer, audio, animation, content indexing, host timer/watcher/subscription/widget/refresh task, or ArtifactStore scan originates from an unloaded tab record.
- Verify pin/bookmark/folder/reminder status consumes no renderer slot; explicit keep-live and other protections remain bounded, expiring, fair, and visible in `why_awake` evidence.
- Verify downloads continue after durable handoff and source renderer destruction.
- Verify lifecycle machinery can never interrupt an active download: drive suspend/unload/ceiling/budget/window-close pressure against renderer-bound downloads and prove the carrier renderer survives untouched until completion or durable handoff (`STAGE-DEC-019`).
- Verify profile service-worker/background activity is separately attributed rather than hidden by zero renderer count.
- Verify session cookies and explicit organization metadata survive unload/restart without keeping page processes alive.
- Lock numerical CPU/memory/latency/resource thresholds only after reproducible representative-hardware baselines are reviewed by the operator.

</topic>
