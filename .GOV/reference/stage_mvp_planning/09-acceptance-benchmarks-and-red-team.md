---
file_id: stage-mvp-acceptance-benchmarks-and-red-team
file_kind: reference-validation-plan
updated_at: "2026-07-19"
status: proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-acceptance-benchmarks-and-red-team" status="proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage acceptance benchmarks and red-team plan

## Planning outcome

Stage completion should be proved at the Handshake product boundary. A rendered page alone is insufficient. The MVP needs measurable evidence for high-volume tabs, Servo-primary behavior, bounded Chromium bootstrap, project intake, isolated sessions, agent control, capture/export, search, recovery, and security.

Exact performance thresholds should be locked after a reproducible baseline on representative operator hardware. The invariants below can be locked now because they express correctness rather than aspirational speed.

## Acceptance suites

### High-volume tab fixture

Minimum fixtures:

- one Stage window with at least 3,000 tab records;
- the same 3,000 records distributed across multiple simultaneously open windows and sessions to prove machine-wide ceilings and fairness;
- nested folders, collapsed branches, labels, bookmarks, Loom links, notes, duplicates, and multiple project/workflow states;
- a mix of active, suspended, unloaded, archived, audio, pre-handoff download, detached-job, unsaved, test, pinned, bookmarked, and explicit keep-live records;
- realistic YouTube/video/reminder URL distributions plus non-video pages;
- synthetic content and an optional operator-supplied sanitized session import kept separate.

Run the same protocol at 1, 10, 100, 1,000, 3,000, and stretch 10,000 records. The scale curve is evidence against per-record idle work; the operator-derived 3,000-plus case remains mandatory and cannot be replaced by synthetic extrapolation.

Measure:

- idle CPU, resident memory, GPU allocation, process count, live/suspended renderer count, and network activity;
- window-record restore time versus selected-tab interactive restore time;
- sidebar frame/input behavior, query latency, folder expansion, selection, bulk mutation, and memory allocation;
- search p50/p95 for exact, prefix, trigram, structured-filter, and full-text queries;
- database/index size and mutation/reindex behavior;
- host-thread and worker wakeups, task/timer/subscription counts, thumbnail/cache allocation, ArtifactStore/maintenance scans, and per-process CPU attribution after a defined settle period;
- cold restore and warm resume behavior under the configured live budget.

Correctness invariants:

- total live renderer count never tracks total tab-record count;
- unloaded/archived tabs produce no page timers, media, script, network, service-worker-on-behalf-of-tab, or renderer processes;
- unloaded/archived records create no per-record host timer, watcher, polling loop, database subscription, UI/accessibility widget, decoded thumbnail, metadata refresher, Loom-sync future, or shared-store scan;
- pin/bookmark/folder/reminder metadata never protects a renderer; explicit keep-live holds are separate, attributable, expiring, fair, and bounded by the global ceiling;
- downloads detach after durable job handoff and do not protect source renderers for the remaining job lifetime;
- profile service workers, background sync/push, browser utilities, GPU processes, workflow workers, and maintenance jobs are attributed and budgeted independently from renderer count;
- only visible sidebar rows allocate row UI;
- tree flattening, filtering, counts, search projections, accessibility projection, and drag/drop do not perform an unbounded full-record walk on every frame or mutation;
- canonical select-all/search/bulk actions include matching rows that were never painted;
- restart restores records before pages and does not create a network storm;
- duplicate grouping never deletes distinct notes, ordering, folder placement, or history automatically.

The 10,000-record stretch fixture is high ROI for detecting accidental O(n) painting, maintenance scans, and headroom regressions, but it must not replace the operator-derived 3,000+ acceptance case.

### Renderer conformance

Run the same Stage-level fixture contract against Servo and the bounded Chromium bootstrap where capabilities overlap:

- navigation/history/redirects/download handoff;
- keyboard, pointer, focus, selection, forms, file chooser, clipboard policy, and IME where supported;
- DOM/script/accessibility observation and stale-target behavior;
- screenshots, viewport changes, zoom, high-DPI, and visual snapshots;
- cookies/cache/storage/service workers/permissions under isolated sessions;
- authentication, common media playback, page capture, print/export capability reporting;
- request blocking, private-network policy, file access, certificate/error pages, and redirects;
- crash, hang, timeout, process termination, and restart recovery.

Servo is promoted by passing the operator-approved primary compatibility/security corpus and resource targets. A complete production-qualified WebView2 Windows slice may ship through `STAGE-WIN-BOOTSTRAP-PROD` after all slice gates pass, but it cannot close the strategic Servo work or the whole Stage WP while Servo completion gates remain open.

### Project asset intake

End-to-end fixtures should cover:

- single video, playlist/channel, forum/gallery images, generic video, direct asset, page, selection, PDF, and supported 3D input;
- anonymous and authorized Stage Session flows;
- existing `hsk.media_downloader.batch@v0`, control, and result schemas;
- captions present, captions absent plus ASR, partial batch, duplicate, retry, cancellation, disk-full, expired session, and intake-unavailable cases;
- ArtifactHandle/hash authority, optional OutputRootDir materialization, ExportRecord, and Lens/Atelier intake linkage;
- tab unload/close and renderer crash while a job continues;
- source tab, job, artifact, transcript, project intake, and Loom relationships.

### Search, translation, and export

- Fuzzy metadata queries work over all 3,000+ records without renderer activation.
- Captured full-text queries return source/capture/provenance and do not refetch pages.
- Live, last-observed, captured, transcript, translated, and Loom text have visible truth labels.
- Local translation works offline for supported installed language pairs; unsupported pairs fail visibly or require explicit provider selection.
- Cloud translation shows content-egress scope and records provider provenance without leaking source text into general logs.
- Markdown uses sanitized captured/readable content and links to the original artifact.
- `Print page` and `Export captured document` remain distinct PDF operations.

### Agent control and visual debugging

- Local and cloud actors use the same typed action/observation contract with distinct identities and capabilities.
- Concurrent actions are attributable and conflicting mutations reject or reconcile through explicit revision rules.
- DOM/accessibility and visual paths can each complete representative fixtures; the fallback path is recorded.
- Navigation invalidates stale element targets.
- Canvas-heavy mock UI is testable through screenshots, pointer/keyboard input, and re-observation.
- CAPTCHA/challenge fixture pauses for operator takeover and resumes from a fresh observation; there is no bypass operation.
- No test opens unexpected foreground windows, steals focus, or hijacks the operator keyboard.
- The visual-debugging suite records screenshots, structured state, logs, traces, errors, health, and reproduction steps.

### Session and secret safety

- Persistent and ephemeral sessions remain isolated for cookies, cache/storage, service workers, and permissions according to capability.
- Clearing one session cannot clear another.
- Cookie edit/import/export round trips supported attributes and reports unsupported attributes.
- JSON export defaults to selected session/domain and masks values in previews.
- Downloader-compatible Netscape cookie artifacts are high sensitivity, not exportable, not written to OutputRootDir, and expire/delete according to policy.
- Secret scanning of logs, events, receipts, screenshots, crash reports, search indexes, and exports finds no raw cookie/token values.

## Baseline and target-lock process

1. Record hardware, OS, power mode, Handshake build, renderer build, dependency pins, session policy, fixture hash, and live-budget settings.
2. Run an idle system baseline and a native-Stage-shell baseline with no live page.
3. Run the 3,000-record fixture with one active renderer and with the proposed default warm budget.
4. Repeat with pinned/bookmarked records but no keep-live holds, then with bounded keep-live/audio/unsaved/test exemptions, proving the distinction.
5. Repeat with the same records distributed across multiple open windows/sessions while retaining one machine-wide renderer/background-work ceiling.
6. Compare Servo and bootstrap Chromium only on equivalent Stage-level scenarios; label capability gaps instead of averaging them away.
7. Repeat enough times to report distribution and outliers rather than one favorable sample, including warm-up, settle, idle sampling, interaction, and recovery phases.
8. Let the operator lock CPU, memory, UI latency, restore, and compatibility targets after seeing the baseline.
9. Store benchmark output as reproducible evidence linked to build and fixture hashes.

No target should be expressed as "better than Chromium" without a named Chromium build, identical fixture, hardware, run protocol, and result. The operator's actual acceptance is low idle resource use with 3,000+ durable tabs and tolerable cold switching.

## Red-team matrix

| Scenario | Failure risk | Minimum control | Proof |
|---|---|---|---|
| Chromium bootstrap becomes permanent | Servo work receives only parity leftovers | whole-WP completion and primary-feature gates remain Servo-bound | lifecycle/taskboard gates cannot close from Chromium evidence alone |
| Shared abstraction becomes lowest-common-denominator | Servo-specific value is suppressed | small invariant kernel plus explicit capability manifest | Servo-only capability fixture remains reachable without UI forks |
| Servo content process escapes | Rust memory safety is mistaken for sandboxing | multiprocess/OS sandbox, least privilege, hostile-site tests | process/permission/network boundary evidence on each target OS |
| One hostile tab harms host or other sessions | shared trust/security domain | renderer containment, session isolation, crash/hang supervision | cross-session and renderer-loss negative tests |
| 3,000 restored tabs create network storm | UI restores pages instead of records | lazy selected/warm-set restore and backpressure | captured network trace after restart |
| Thousands of pinned tabs defeat the live ceiling | organization metadata is treated as keep-live | pin/bookmark remain metadata; explicit expiring keep-live consumes budget | pinned-versus-keep-live scheduler fixture |
| Renderer count is low but host CPU still scales with records | per-record timers, watchers, subscriptions, thumbnail decode, recounts, or refresh jobs | zero-per-record dormant contract plus bounded shared event queues | 1-to-10,000 scale curve with wakeup/task/allocation evidence |
| Multiple windows multiply renderer and worker budgets | each window enforces only a local cap | machine-wide ceiling, reserved interactive capacity, and fair window/session shares | simultaneous multi-window resource fixture |
| Service workers or browser utilities remain active after detach | renderer-only accounting claims false quiescence | profile/process attribution, separate budgets, quarantine/clear controls | background-process and network attribution suite |
| Exemptions grow without limit | audio/dirty/test/model holds bypass eviction forever | ceilings, expiry/renewal, fairness, visible owner/reason, over-budget disposition | exemption-saturation property suite |
| Artifact or maintenance scans enter the tab hot path | startup/sidebar events trigger full workspace scans | shared-subsystem work is independently scheduled, incremental, rate-limited, and never per-tab | no-scan startup/sidebar trace plus maintenance stress fixture |
| Sidebar virtualizes rows but bulk action uses visible slice | only loaded rows are changed | canonical query/ID-set action source | offscreen matching rows mutate in test |
| Sidebar paints only visible rows but still walks all records | hidden O(n) flatten/count/filter/accessibility work causes CPU spikes | incremental invalidation, bounded paging, debounce/cancel, allocation budgets | mutation/filter/drag/accessibility profiling at 3,000 and 10,000 records |
| Tab unload loses unsaved form/test work | overly aggressive eviction | visible exemption/reason and conservative dirty-state policy | form/test fixtures survive or warn before loss |
| Downloads depend on renderer | unload/click cancellation loses assets | detach into workflow job before unload | kill renderer during active download |
| Cookie export leaks account access | values in logs/files/screenshots | high-risk gate, scope, encryption, redaction, secret scans | negative secret corpus and inspection |
| Stage folders duplicate Loom graph | divergent tags/relationships | explicit ownership and ID references | rename/link/backlink consistency tests |
| Search wakes pages | CPU/network grows with tab count | metadata/captured-artifact indexes only | zero page activity during query/reindex |
| Search returns stale capture as live fact | operator/model trusts obsolete data | source-kind, capture-time, and staleness labels | mixed live/captured/translated fixture |
| Translation overwrites source | provenance and meaning lost | derived artifact and immutable source | source hash unchanged after translation |
| Readability output carries active content | script injection in export/view | sanitization, CSP/isolated viewer, untrusted-input tests | malicious HTML corpus |
| Agent acts on stale DOM | wrong click or destructive action | navigation/document revision on every target | forced mutation/navigation race tests |
| Parallel models fight over one tab | nondeterministic operator state | actor IDs, revisions, leases/coordination, recoverable receipts | concurrent conflicting action suite |
| Renderer fallback is silent | security/feature meaning changes | visible explicit engine provenance and no-silent-fallback rule | capability-denial fixture |
| Product topology differs from plan | planning invents duplicate modules | mandatory product-worktree inspection before contract lock | source topology and proposed-to-existing component map |

## Promotion and completion gates

Planning proposal:

- `STAGE-WIN-BOOTSTRAP-PROD`: the complete WebView2-backed Windows product slice passes its feature, authenticated-site disposition, security, reliability, accessibility, packaging, backup, support, and evidence gates; it may ship without closing the umbrella WP.
- `STAGE-SERVO-RESTRICTED-ALPHA`: trusted/allowlisted Servo scope passes embedding, isolation, compatibility subset, cleanup, recovery, packaging, rollback, diagnostics, and restriction-bypass-negative gates.
- `STAGE-SERVO-ARBITRARY-WEB`: effective Windows content-process sandbox plus independent escape-oriented proof; currently `SECURITY_BLOCKED` by upstream capability.
- `CHROMIUM_RETIRE_OR_DEV_ONLY`: reached only after Servo default evidence and explicit operator decision.
- `STAGE-WP-COMPLETE`: current approved scope, complete legacy-source dispositions, all approved requirements, strategic Servo gates, documentation, diagnostics, current active-WP integration, and all official microtasks are complete and synchronized with no legacy Stage runtime authority.

## Remaining evidence gaps

- Product/active-WP source was inspected read-only on 2026-07-19; exact approved baseline commits, final file ownership, and runnable future Stage entrypoints still require binding before official MT generation.
- Servo Windows arbitrary-web sandbox readiness is a confirmed current capability blocker and needs future upstream/runtime proof.
- WebView2 embedded Google authentication conflicts with the primary authenticated YouTube workflow; a supported path or explicit operator-approved limitation is required before Windows production promotion.
- Representative site corpus and exact compatibility threshold need operator approval.
- Real hardware baselines and numerical performance targets do not yet exist.
- The high-volume scale curve, multi-window ceiling, dormant host-work inventory, service-worker attribution, and pin-versus-keep-live prototype evidence do not yet exist.
- Translation language coverage/quality and Servo print/PDF behavior need implementation-lane evaluation.

</topic>
