---
file_id: stage-full-feature-product-requirements
file_kind: reference-product-requirements
updated_at: "2026-07-19"
status: research-hardened-proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-full-feature-product-requirements" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage full-feature product requirements

## Purpose and boundary

This document defines the browser-product surface that the later Stage specification, consolidated WP stub, refinement, and microtasks must cover. It does not edit those authority surfaces or create executable microtasks. Stable requirement IDs and machine-readable trace fields live in `requirements-and-traceability.yaml`.

`Full feature` means the planning corpus covers the complete intended product, including ordinary browser workflows, Handshake-native asset intake, model operation, production operations, failure recovery, accessibility, and no-context-model usability. It does not mean every feature must ship in the first release slice.

## Release slices

| Slice | Meaning | Closure rule |
|---|---|---|
| `STAGE-WIN-BOOTSTRAP-PROD` | Production-qualified Windows Stage using the bounded WebView2 adapter. | May ship only when its complete feature, security, recovery, packaging, compatibility, and evidence set passes. It does not close the umbrella WP or make Chromium a coequal strategic backend. |
| `STAGE-SERVO-RESTRICTED-ALPHA` | Servo integration limited to explicitly trusted/allowlisted fixtures and content. | Requires Servo embedding, isolation, storage cleanup, compatibility, crash, packaging, and rollback proof. It must remain visibly restricted. |
| `STAGE-SERVO-ARBITRARY-WEB` | Servo permitted for arbitrary hostile web content on Windows. | Blocked until effective default-deny content-process sandboxing and escape-oriented validation pass. Multiprocess alone is insufficient. |
| `STAGE-WP-COMPLETE` | The later unified Stage packet is complete. | Requires current approved scope, explicit legacy-source dispositions, all operator outcomes, full product scope, Servo strategic completion gates, documentation, diagnostics, current integrations, and synchronized official MT/WP/task state with no legacy Stage runtime authority. |

The Windows bootstrap slice resolves near-term production delivery without weakening the operator-locked Servo direction. A WebView2 feature may be production-grade and still remain a temporary adapter implementation.

## Native browser shell and navigation

Stage owns browser chrome because WebView2 does not provide a full Edge browser UI or its profile identity/sync, favorites, history, translation, reader, or settings surfaces.

The native shell includes:

- a security-aware omnibox with URL/search disambiguation, configurable search providers, paste-and-go/search, history/bookmark suggestions, invalid-input handling, and no silent transmission of sensitive local strings;
- back, forward, reload, hard reload where supported, stop, home/new-tab, duplicate tab, reopen closed tab/window, move tab, pin, mute, close, close-others/right, and restore actions;
- per-tab joint session history distinct from profile visit history; back/forward restore must preserve engine-supported form, scroll, frame, and same-document state without treating profile history as session authority;
- native find-in-page, zoom, page/fullscreen transitions, media/audio indicators, picture-in-picture capability reporting, context menus, status/hover URL display, popup/new-window handling, and optional developer tools behind an operator capability;
- keyboard, command-palette, stable action-ID, mouse, touchpad, IME, drag/drop, clipboard, file-chooser, and accessibility routes;
- visible page identity, origin, TLS/certificate state, permission activity, engine/provenance, session/private state, model-control lease, capture state, and download state.

Blocked engine pages such as `edge://settings`, `edge://favorites`, `edge://history`, and `edge://extensions` are never treated as available product surfaces. Stage supplies its own native equivalents where the product requires them.

## Windows, tabs, history, bookmarks, and onboarding

Stage maintains durable canonical records separately from live renderers:

- multiple windows, per-window active tab and sidebar state, tab move/copy between windows, deterministic focus restoration, and crash-safe window restore;
- tab folders/groups, nested organization, ordering, color labels, pins, bookmarks, notes, opened-from relations, watch/reminder state, and Loom references;
- recently closed tabs/windows with bounded retention, explicit removal, and restore ordering;
- profile visit history with time, URL/title, source session, visit transition, search/filter/delete/clear controls, and privacy-mode exclusion;
- bookmark and folder import/export using explicit versioned formats; browser/HTML/URL-list intake must report duplicates, malformed entries, unsupported metadata, and partial success;
- onboarding for an existing 3,000-plus-tab corpus without opening every page, creating a network storm, or losing source hierarchy and ordering;
- duplicate detection as an operator-visible suggestion using normalized URL/content facts; it never silently deletes distinct history, notes, folder placement, or artifacts;
- canonical-set search/select-all/bulk operations over records that were never rendered in the sidebar.

## Browser services

The plan covers all browser-host services rather than assuming the engine UI owns them:

- a download manager with destination policy, progress, pause/resume/cancel/retry, collision handling, checksum/provenance, disk-full and permission failures, malware/policy/scan failures, interrupted-download recovery, and handoff to Media Downloader when the workflow becomes a governed background job;
- permission broker for camera, microphone, geolocation, notifications, clipboard, file-system access, MIDI/devices, local fonts, screen capture, window management, automatic downloads, popups, protocol handlers, and any new engine-reported permission kind;
- site settings per origin/session with allow-once, allow-session, persistent allow/deny, expiry, reset, and audit receipt;
- certificate error, client-certificate, HTTP authentication, proxy authentication, external-scheme, captive-portal, and enterprise-root flows with no automatic security bypass;
- password/autofill/passkey/WebAuthn policy. Stage does not invent a raw password vault. Credential UX is engine/system/broker mediated, and unsupported paths fail visibly;
- clear-browsing-data UI by profile, data kind, time range, and previewed impact; clearing one profile must not clear another;
- notifications and background-service behavior only when the adapter can enforce the Stage policy. Missing WebView2 push, Web Payment, or periodic background-sync support is reported as a capability gap rather than emulated silently;
- safe-browsing, download scanning, tracking prevention, ad/filter lists, per-site exceptions, update provenance, rollback, and diagnostics;
- print page, save/capture, view source, readable view, translation, spelling/language, PDF viewing, media/codec/DRM, WebRTC/screen-share, service-worker, WebSocket/WebTransport, and offline/network-transition compatibility matrices.

A general third-party browser-extension marketplace remains outside the current initial scope. Stage Apps are a separate signed/trusted package class with their own lifecycle and bridge policy.

## Sessions, profiles, privacy, and portability

Stage distinguishes:

- ordinary persistent profiles;
- strong-isolation profiles with separate user-data/storage/process domains;
- ephemeral profiles with verified deferred cleanup;
- operator/model project scopes and credential/trust classes;
- explicit, partial cross-engine import/export with loss reports.

Cookies, cache, local/session storage, IndexedDB, service workers, permissions, credentials, downloads, visit history, and browser preferences have named ownership, retention, clear, backup, and migration behavior. Raw browser profile directories are materialized runtime state, not canonical backup authority.

Cookie view/edit/import/export is explicit, scoped, redacted, and high sensitivity. A governed session-credential lease is preferred for Downloader. JSON or Netscape cookie export is an exceptional operator action, not a presumed routine authentication solution.

## Handshake-native workflows

The full product implements the current approved Stage functions. Similarity to older Stage plans does not preserve their implementation or contract shape:

- Stage Capture through `StageCaptureCoordinator`, including renderer-state page/selection/readable/screenshot/PDF/media/document/3D capture with shared canonical `ArtifactHandle`, hashes, navigation/engine provenance, completeness labels, sanitizer/extractor versions, and partial-result facts;
- separate independent archival acquisition, including WARC when replay-grade evidence is intended; a renderer capture never claims archive completeness;
- page media, playlist/channel, gallery/forum, generic video, and direct-asset download actions routed through Media Downloader and ArtifactStore;
- destination project and Lens/Atelier/CKC intake with batch progress, partial failure, retry, deduplication, cancellation, and tab/job/artifact lineage;
- captions-first and ASR fallback with preserved media-to-transcript/timing lineage;
- tab/folder/bookmark/watch relations linked to Loom without creating a duplicate Stage knowledge graph;
- fuzzy metadata and captured-content search that does not activate or refetch unloaded pages;
- local-first translation with model/package provenance and explicit cloud egress; source content stays immutable;
- readable Markdown, captured-document PDF, print-page PDF, and governed Export/Materialize behavior;
- a newly specified editor-to-Stage and capture/embed-back public workflow if retained by current requirements, without WP-12 Stage aliases, panes, wire types, schemas, adapters, connectors, or mockups.

## Resource lifecycle and failure recovery

The canonical registry supports 3,000-plus records while one machine-wide ceiling plus fair bounded session/window shares controls live/suspended renderers and background work. Scheduler decisions are deterministic and expose admission/eviction reasons, `why_awake`, protected-work owners, expiries/renewals, fairness, over-budget disposition, memory pressure, restore rate, network rate, capture/index budgets, and accounting. Pin/bookmark/folder/reminder metadata is never a renderer protection; explicit keep-live is separate and budgeted. Downloads protect the source renderer only until durable job handoff.

Unloaded and archived records allocate no renderer, per-record host timer/watcher/poller/subscription, UI/accessibility widget, decoded thumbnail, refresh future, Loom-sync task, or shared-store scan. Folder/search/count/accessibility/drag projections use incremental invalidation or bounded paging rather than per-frame/per-event full-record walks. Service workers, utilities, GPU processes, workflow jobs, capture/hash/index/thumbnail work, migrations, backup, diagnostics, and ArtifactStore maintenance are separately attributed, bounded, and kept outside the tab/sidebar hot path.

`MemoryUsageTargetLevel.Low` is not equivalent to suspension: scripts may continue. A discarded tab is not proof of zero profile-level service-worker activity. Stage tracks record, renderer attachment, page lifecycle, navigation, and profile background activity separately.

Recovery includes browser/renderer/GPU/network/process crashes, hangs, forced termination, event loss/reorder, corrupt/locked profile, database outage, disk full, low memory, runtime update, interrupted migration, orphan process/profile cleanup, safe-mode restore without eager navigation, quarantine, crash-loop breaker, and state-preserving restart.

## Accessibility, localization, and manual

Stage ships keyboard-only completion, stable focus order, screen-reader/UIA semantics across native and embedded content, high contrast, text scaling, zoom, reduced motion, DPI/multi-monitor behavior, IME, and visual-debug proof. Accessibility must be tested with the existing visual/Argus path plus focused assistive-technology checks.

Localizable strings use stable resource IDs, BCP-47 locale handling, fallback/plural rules, RTL mirroring, locale-aware sorting/search, pseudo-localization, long-string fixtures, and localized errors/manual content. Engine locale and `Accept-Language` behavior are explicit per session.

The UserManual covers installation, first run, navigation, high-volume organization, sessions, authentication limitations, downloads/capture/intake, model control/takeover, permissions, privacy, updates, backup/restore, diagnostics, safe mode, recovery, known capability gaps, and exact startup/test commands. A no-context model can discover stable action IDs, state, receipts, screenshots, and evidence without chat history.

</topic>

<topic id="stage-full-feature-non-goals" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Full-feature scope edges

The full planning corpus does not silently absorb:

- the general model/provider runtime, ToolGate, ProcessOwnershipLedger, Flight Recorder, EventLedger, Argus, or promotion authority owned by WP-1;
- editor document authority or general editor embed-back owned by WP-12;
- ArtifactStore byte authority, retention, materialization, or garbage collection;
- Media Downloader or ASR execution internals;
- Loom knowledge-graph authority;
- CKC, Lens, or Atelier domain authority;
- general browser-extension marketplace support, bulk web crawling/mirroring, Docling structured-PDF conversion, Stage Studio authoring, or advanced 3D editing/collaboration unless the operator later expands scope.

These systems expose versioned integration contracts and fixtures; Stage does not duplicate their persistence or process ownership.

</topic>
