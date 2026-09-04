---
file_id: stage-mvp-decision-register
file_kind: reference-decision-register
updated_at: "2026-09-01"
status: research-hardened-supersession-locked
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-mvp-decision-register" status="research-hardened-supersession-locked" version="v0.7" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-09-01">

# Stage planning decision register

## Operator-locked direction

| ID | Decision | Status |
|---|---|---|
| STAGE-DEC-001 | Fold the older Stage-owned browser/webviewer, portability, and ASR-lineage stubs into `WP-1-Handshake-Stage-MVP-v1`. | LOCKED |
| STAGE-DEC-002 | Plan one massive umbrella WP with more than 100 microtasks. | LOCKED |
| STAGE-DEC-003 | Chromium becomes working and usable first but remains minimal. | LOCKED |
| STAGE-DEC-004 | Servo/Rust is the strategic main Stage webviewer and receives the main feature effort. | LOCKED |
| STAGE-DEC-005 | Do not plan two co-equal backends that divide feature work and attention. | LOCKED |
| STAGE-DEC-006 | Rewrite the master-spec Stage topic only after initial planning concludes. | LOCKED |
| STAGE-DEC-007 | Use a `.GOV/reference` workspace with topic indexes and a research subfolder for iteration. | LOCKED |
| STAGE-DEC-008 | Stage's primary initial operator workflow is collecting videos, images, and assets from the web and ingesting them into Handshake projects for Lens/Atelier. | LOCKED |
| STAGE-DEC-009 | Stage must support windows containing thousands of tabs, including the operator's current 3,000+ YouTube-tab reminder/watch queues, without thousands of live renderers consuming CPU. | LOCKED |
| STAGE-DEC-010 | Slow restoration of inactive tabs is acceptable when it materially reduces idle CPU and memory use. | LOCKED |
| STAGE-DEC-011 | Stage requires a per-window tab sidebar, tab folders/groups, color labels, bookmarks, and tags with a defined Loom interaction. | LOCKED |
| STAGE-DEC-012 | Stage requires cookie inspection/editing and JSON export. | LOCKED |
| STAGE-DEC-013 | Stage must be steerable by local and cloud AI through visual and structured interaction paths, including website interaction and self-testing web builds or GUI mockups. | LOCKED |
| STAGE-DEC-014 | Stage features and usage instructions must ship in Handshake's internal UserManual/model manual surfaces. | LOCKED |
| STAGE-DEC-015 | Stage requires translation, Markdown/PDF export, fuzzy tab/bookmark/folder/tag search, and searchable captured website content. | LOCKED |
| STAGE-DEC-016 | Complete and harden full-feature planning/research first; defer Master Spec rewrite, old-stub folding, replacement WP/refinement, and official MT creation to the later authority pass. | LOCKED |
| STAGE-DEC-017 | The current Stage direction supersedes every older Stage-specific implementation and product direction, including already-built prototypes, adapters, connectors, schemas, routes, panes, and mockups. Old Stage assets are non-authoritative salvage inputs only; they impose no compatibility, reuse, migration, or UX constraint unless the current Stage contract explicitly selects one. Shared non-Stage Handshake authorities and real operator data are outside this supersession boundary. | LOCKED |
| STAGE-DEC-018 | Keep the existing ArtifactStore as shared global Handshake infrastructure. Stage exposes a Stage-specific capture/intake workflow through `StageCaptureCoordinator`, consumes shared opaque `ArtifactHandle`s, and does not create or present a Stage-specific artifact store. | LOCKED |
| STAGE-DEC-019 | Download continuity is unconditional. No Stage lifecycle mechanism — tab suspend, freeze, throttle, unload, archive, resource budget, machine-wide ceiling, window/tab close, session dormancy, or app-level power saving — may stop, pause, freeze, or make dormant an active download. A download runs to completion, explicit operator/job-level cancellation, or genuine external failure (disk full, network loss, server error). While a download is renderer-bound and not yet handed off to an independent job, its carrier renderer is exempt from suspension and unloading until the download completes or durable handoff succeeds; this exemption is visible in `why_awake` evidence but is not time-bounded and never expires while the download is active. | LOCKED |
| STAGE-DEC-020 | The core Stage product framing is simple: Stage is a normal browser whose single defining performance requirement is that 3,000+ tabs do not tax the CPU. Every other behavior defaults to what a normal browser does, inherited from the embedded engine and standard browser conventions, unless another Handshake module's contract explicitly overrides it. Planning, requirements, and future WP/MT authoring must not invent bespoke replacements for ordinary browser behavior; complexity is only justified by the tab-scale requirement, a Handshake-module integration, or a proven security/safety need. | LOCKED |
| STAGE-DEC-021 | The WebView2/Chromium bootstrap lives or dies on authenticated YouTube. The governed session-import path must prove a working logged-in YouTube journey in WebView2: logged-in state on youtube.com, playback, playlist/channel access, and authenticated download handoff. If that proof fails, the Chromium bootstrap and the `STAGE-WIN-BOOTSTRAP-PROD` release slice are abandoned and all Stage effort focuses on Servo. No compensating second browser lane (full-browser compatibility route or similar) is built to rescue the bootstrap. | LOCKED |
| STAGE-DEC-022 | `STAGE-WP-COMPLETE` is the one true "Stage is finished" definition: every release-slice proof complete and Chromium retired or demoted. No intermediate slice, bootstrap release, or alpha may ever be represented as Stage completion. | LOCKED |
| STAGE-DEC-023 | Stage product persistence uses Handshake-managed embedded SurrealDB/EventLedger exclusively through the shared application-bootstrap storage handle and a typed Stage-owned repository boundary. Stage must not introduce Turso, libSQL, SQLite, PostgreSQL, a feature-private database engine, fallback storage, or dual-write authority. Browser-engine profile files remain isolated engine materialization rather than canonical Stage database or backup authority. | LOCKED |

`STAGE-DEC-017` resolves the earlier compatibility assumption around the WP-12 Stage prototype. The new Stage module is canonical. An editor-to-Stage workflow may be designed as a new public integration, but the existing `StagePane`, routes, wire types, storage, capability IDs, connectors, adapters, and mockups do not receive a survival guarantee merely because they exist or are already built.

`STAGE-DEC-018` resolves the ArtifactStore naming and ownership question. ArtifactStore remains the shared byte/manifests/hash/retention service used across Handshake. The Stage-facing product/process name is Stage Capture; `StageCaptureCoordinator` owns capture intent, job correlation, capture lineage, and handoff only. It cannot own bytes, manifests, hashing, retention, garbage collection, materialization, raw paths, or a new artifact-handle encoding.

`STAGE-DEC-019` supersedes every earlier "bounded pre-handoff protection" formulation for downloads. Earlier corpus text allowed a renderer carrying a not-yet-handed-off download to become unloadable after a bounded protection window; that is no longer permitted. Lifecycle machinery is never a valid cause of download interruption. Explicit operator cancellation, job-level cancellation through the download/job surface, and genuine external failures remain valid terminal outcomes; streaming backpressure that briefly slows a transfer for I/O reasons is not lifecycle dormancy and remains legal, but it must never convert into a silent stop.

`STAGE-DEC-020` is the anti-overcomplication rule. It does not delete existing hardened planning, but it governs how planning text is framed and how future requirements are authored: the normal-browser baseline is the default, engine-inherited behavior is preferred over bespoke re-specification, and the burden of proof is on complexity. When a planning artifact proposes Stage-specific machinery for something a normal browser already does acceptably, it must name the tab-scale, Handshake-integration, or security justification or be simplified.

`STAGE-DEC-021` resolves the authenticated Google/YouTube open question and conditions the bootstrap. Google policy blocks embedded login in WebView2, so governed session import is the only candidate path; it is now the bootstrap's viability test rather than one option among several. A time-boxed spike must prove the four-part authenticated YouTube journey. PASS keeps the `STAGE-WIN-BOOTSTRAP-PROD` slice alive; FAIL abandons the Chromium bootstrap entirely — no full-browser side lane, no partial retention as a co-equal backend — and reallocates all Stage effort to Servo (restricted alpha first, arbitrary-web after the sandbox gate). This supersedes the `FULL_BROWSER_COMPATIBILITY_ROUTE` as a rescue path for the bootstrap; that route survives only as a possible far-future option unrelated to bootstrap viability. `STAGE-REC-019` (production-qualified WebView2 slice) is now conditioned by this gate.

Spike status (see research note `STAGE-RW-021`): a Rust WebView2 harness was built and run on 2026-07-20 against the operator's real Google/YouTube session (read from the Firefox `default-release` profile). Empirical result: governed session import into WebView2 **works** — with the essential ~40 `.youtube.com` auth cookies injected, YouTube reports logged-in (`LOGGED_IN=true`, account avatar, notification badge), and the auth-gated `/feed/subscriptions` page rendered as a logged-in YouTube Premium session (screenshot visually confirmed). Gate parts 1-3 PASS; part 4 (authenticated download handoff) not tested. This **corrects the earlier prediction of FAIL/FRAGILE**: Google's embedded-webview block targets interactive login, not the presentation of already-valid session cookies, so cookie import sidesteps it. Therefore `STAGE-DEC-021` does NOT fire on current evidence and the `STAGE-WIN-BOOTSTRAP-PROD` slice stays alive. Two engineering constraints surfaced: (1) session import must scope cookies to the target site's essential set — importing the full cross-property Google jar triggers HTTP 413; (2) Firefox is the practical unencrypted session source on this machine. One open question remains — session DURABILITY over time under DBSC/cookie-refresh (not yet soak-tested); that governs the re-import UX, not initial viability, and Servo inherits the same approach and question. Harness lives at `C:\Handshake_Stage_Spike` (outside the governance kernel, on C:).

`STAGE-DEC-022` locks the closure semantics that were previously pending approval: intermediate slices are delivery milestones, never completion claims. Whole-WP closure requires Chromium retired or demoted, consistent with the bootstrap-feature-freeze and promotion-gate topics.

`STAGE-DEC-023` locks the Stage persistence boundary. Stage-owned durable records use typed SCHEMAFULL SurrealDB tables, indexes, assertions, authenticated record-user permissions, SurrealKit rollouts, and EventLedger transitions through an injected `StageRepository`; application bootstrap owns the canonical embedded `SurrealStorage` handle. ArtifactStore remains byte/manifests/hash/retention/materialization authority, while Stage stores opaque artifact references and lineage. Engine-native cookies, cache, local storage, IndexedDB, service workers, DRM state, and similar profile data may remain in isolated engine profile directories, but those files cannot become a second product database or canonical backup source.

## Working recommendations requiring later confirmation

| ID | Recommendation | Rationale | Status |
|---|---|---|---|
| STAGE-REC-001 | Treat Chromium as a removable bootstrap adapter rather than a permanent product backend. | Prevents feature-parity work and bootstrap permanence. | PROPOSED |
| STAGE-REC-002 | Make headless-agent and restricted-document operation Servo profiles. | Preserves one strategic engine while supporting distinct capability policies. | PROPOSED |
| STAGE-REC-003 | Forbid silent engine fallback and automatic cross-engine profile sharing. | Prevents invisible semantic, privacy, and evidence drift. | PROPOSED |
| STAGE-REC-004 | Make Stage privacy and tracker defense a core requirement rather than an optional enhancement. | Aligns with the operator's browser concerns and Servo request-interception opportunity. | PROPOSED |
| STAGE-REC-005 | Keep a small renderer-neutral Stage kernel contract without demanding feature parity. | Protects host-domain invariants while allowing Servo-specific capabilities. | PROPOSED |
| STAGE-REC-006 | Use aggressive tab suspension and renderer unloading rather than aggressive HTTP-cache clearing. | Unloading releases live page CPU/memory; indiscriminate cache clearing increases reload work and does not stop active page execution. | PROPOSED |
| STAGE-REC-007 | Keep folder hierarchy as Stage operational organization while using Loom tags, mentions, backlinks, and LoomBlocks for durable project knowledge. | Avoids two competing knowledge graphs while preserving fast browser-like tab management. | PROPOSED |
| STAGE-REC-008 | Prefer governed session credential handoff to Media Downloader over raw cookie export; retain explicit JSON export as a high-risk operator action. | Supports downloading without routinely materializing reusable session secrets. | PROPOSED |
| STAGE-REC-009 | Use direct WebView2 through `webview2-com` as the Windows-first embedded Chromium bootstrap. | It supplies first-party profiles, request policy, Chromium process isolation, failure events, CDP access, and Evergreen servicing while remaining isolatable behind the adapter. | PROPOSED |
| STAGE-REC-010 | Keep Wry prototype-only; use Chrome-for-Testing only as an isolated headless validation worker; escalate to CEF only after a failed WebView2 composition spike or a proven offscreen requirement. | This minimizes bootstrap and packaging burden while preserving evidence-backed escape hatches. | PROPOSED |
| STAGE-REC-011 | Permit Servo this month only as a trusted/allowlisted-content alpha after embedding, isolation, compatibility, recovery, and operational gates pass. | Current Servo Windows content processes are explicitly unsandboxed; availability of multiprocess binaries is not arbitrary-web security proof. | PROPOSED |
| STAGE-REC-012 | Keep arbitrary-web Servo on Windows blocked until an effective default-deny content-process sandbox and independent negative tests pass. | Prevents a calendar target from weakening the hostile-web trust boundary. | PROPOSED |
| STAGE-REC-013 | Shape the canonical model-control surface around WebDriver BiDi concepts, with CDP, Playwright, WebView2, and Servo protocols confined to versioned adapters. | Preserves stable Stage identities and cross-engine semantics without depending on Chromium tip-of-tree compatibility. | PROPOSED |
| STAGE-REC-014 | Split lifecycle into durable record, runtime attachment, page lifecycle, navigation, and control-lease facets. | A single `sleeping` state cannot represent discard, crash, restore, navigation, or operator/model ownership safely. | PROPOSED |
| STAGE-REC-015 | Require durable pre-dispatch action intent and `OUTCOME_UNKNOWN_RECONCILE_REQUIRED` for uncertain non-idempotent side effects. | Prevents blind retries after timeouts or process loss. | PROPOSED |
| STAGE-REC-016 | Make all bulk operations use a frozen canonical-registry target set, never visible sidebar rows. | Prevents partial actions and false completion at 3,000-plus-tab scale. | PROPOSED |
| STAGE-REC-017 | Distinguish current-renderer capture, independent acquisition/archive, and selection/media extraction. | Preserves authenticated renderer evidence while avoiding false archival claims. | PROPOSED |
| STAGE-REC-018 | Reuse shared WP-1 orchestration/process/telemetry services, supersede the WP-12 Stage prototype, and keep CKC downstream through newly approved public contracts. | Prevents duplicate authority without allowing older Stage implementation shapes to constrain the new Stage design. | PROPOSED |
| STAGE-REC-019 | Permit a complete production-qualified WebView2 Windows release slice without treating it as strategic Servo or whole-WP completion. | Provides a truthful near-term delivery path while preserving the Servo direction and security block. | PROPOSED |
| STAGE-REC-020 | Make the full native Stage module canonical and supersede the WP-12 Stage pane/module placeholder as a Stage implementation. Any editor-to-Stage route or embed-back workflow is specified anew against the current Stage public contract. | Resolves duplicate worksurface topology without granting legacy UI or wire contracts continuing authority. | LOCKED_BY_STAGE_DEC_017 |
| STAGE-REC-021 | Treat authentication/session acquisition as an explicit per-site capability: engine-native login, external-user-agent OAuth, governed session import, full-browser compatibility route, or unsupported/security-blocked. | WebView2/Google policy prevents assuming embedded Google/YouTube login, and OAuth tokens are not website cookies. | PROPOSED |
| STAGE-REC-022 | Build browser-agent security around hostile-content provenance, source-to-sink control, consequence classes, least privilege, confirmation/watch/takeover, and non-exfiltration proof rather than a classifier-only gate. | Prompt injection cannot be reliably solved by detecting malicious strings alone. | PROPOSED |
| STAGE-REC-023 | Expand the initial 18-lane/186-slot seed to a 23-lane blueprint and freeze official IDs only after requirement, file-conflict, migration, gate, and command validation. | Full browser features and production persistence/operations were hidden inside overly broad lanes. | PROPOSED |

## Open decisions

- Operator confirmation of the proposed direct-WebView2 Windows bootstrap and its supported operating-system matrix.
- Minimum bootstrap feature boundary.
- Measured acceptance thresholds and Chromium retirement condition after the WebView2/Servo prototypes run.
- Timing and upstream acceptance criteria for lifting the Windows arbitrary-web Servo sandbox block.
- Final `adblock-rust` evaluation, filter-list lifecycle, user rules, and request-policy composition after adapter coverage tests.
- Profile persistence and explicit authenticated-session migration between engines.
- Servo fork, patch-queue, LTS pinning, and upstream-contribution policy.
- Compatibility corpus representing Handshake's real browsing, authentication, media, and agent workflows.
- Exact live/suspended/unloaded/archived tab lifecycle, thresholds, exemptions, and resource budgets.
- Stage-folder versus Loom-tag/collection authority and synchronization rules.
- Cookie JSON schema, encryption, redaction, import behavior, and downloader credential-lease contract.
- Translation backends, cloud-egress controls, and source/translation artifact model.
- Website-content indexing trigger: loaded page, explicit capture, download ingest, or a combination.
- Whether CEF offscreen rendering is actually required after the WebView2 child/composition spike.
- Exact current-renderer snapshot bundle and independent WARC/archive product boundary.
- Final resolution of active WP-1 process-control blockers and WP-12 legacy Stage removal, collision cleanup, and optional real-data import before Stage activation.
- RESOLVED 2026-07-20: `STAGE-WP-COMPLETE` closure semantics locked by `STAGE-DEC-022`; `STAGE-WIN-BOOTSTRAP-PROD` viability now conditioned by `STAGE-DEC-021`. Servo restricted/arbitrary-web slice boundaries remain as planned.
- RESOLVED 2026-07-20: authenticated Google/YouTube path locked by `STAGE-DEC-021` — governed session import is the bootstrap viability test; failure abandons the Chromium bootstrap in favor of Servo.
- Project-wide Windows installer/update/signing baseline for Stage assets.
- Exact approved integration commits plus future command bindings.
- Measured performance/resource/compatibility/RPO/RTO/evidence-freshness/support targets.

</topic>
