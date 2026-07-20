---
file_id: stage-rw-005-008-browser-control-security-and-capture
file_kind: reference-research-note
updated_at: "2026-07-19"
status: primary-source-deep-review-complete-validation-pending
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-rw-005-008-browser-control-security-and-capture" status="primary-source-deep-review-complete-validation-pending" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# STAGE-RW-005 through 008: browser control, privacy, sessions, diagnostics, and capture

## Selected cross-engine control model

Stage should expose a browser-engine-neutral, WebDriver-BiDi-shaped command/event protocol. WebDriver BiDi supplies the useful cross-browser concepts: command IDs, success/error results, event subscriptions, browsing-context IDs, user contexts, navigation IDs, and background context creation. Chromium CDP, Playwright, and Servo protocols remain private versioned adapters. CDP is Chromium-specific and its tip-of-tree contract is not backward compatible; Servo's current WebDriver BiDi pull request is still draft and cannot be treated as a production dependency.

The Stage adapter capability manifest should at minimum identify:

```yaml
capabilities:
  - create_background
  - activate
  - context_tree
  - semantic_snapshot
  - screenshot
  - input_actions
  - network_observe
  - network_intercept
  - download_observe
  - user_contexts
  - freeze
  - discard
  - resource_metrics
  - process_introspection
```

Unsupported capabilities fail explicitly. Stage cannot silently change adapters, focus the UI, or lower assurance.

## Durable identity and lifecycle

A Stage tab ID survives unload, process loss, application restart, and restore. Engine context, target, process, document, locator, and navigation IDs are ephemeral and generation-scoped. One ambiguous `sleeping` state is rejected.

```text
record_state:
  OPEN | CLOSED | ARCHIVED

runtime_attachment:
  DETACHED_COLD | STARTING | ATTACHED | CRASHED | STOPPING

page_lifecycle:
  ACTIVE | PASSIVE | HIDDEN | FROZEN | DISCARDED | TERMINATED | UNKNOWN

navigation_state:
  IDLE | STARTED | COMMITTED | INTERACTIVE | COMPLETE | FAILED

control_state:
  UNLEASED | MODEL_LEASED | OPERATOR_LEASED |
  BLOCKED_OPERATOR | CANCELLING
```

This model aligns persistent 3,000-plus-tab records with a bounded live renderer set. Restore is prioritized and lazy; resource thresholds must come from measured target-hardware baselines.

## Agent action protocol and receipts

Stage owns browser observation, navigation, input, tab focus, takeover, and browser postconditions. WP-1 owns model lanes, ToolGate, consent, model selection, process ownership, recovery, promotion, EventLedger, Flight Recorder, and Argus integration. Stage must reuse those services after their active acceptance blockers are cleared.

Before a side effect, Stage durably records an intent containing:

- action, attempt, idempotency, causation, correlation, and trace IDs;
- WP-1 model-lane/run identity;
- stable Stage session/tab IDs and expected state revision;
- adapter name/version/capabilities and engine generation;
- ephemeral engine context ID;
- target, parameter digest, ToolGate/consent decision, and focus policy;
- deadline and pre-observation hash.

The terminal receipt records dispatch and result. Timeout or crash with uncertain effect becomes `OUTCOME_UNKNOWN_RECONCILE_REQUIRED`; a non-idempotent action cannot retry until Stage reconciles the page and durable record.

Observation receipts combine semantic structure, element references, bounding boxes, viewport/actionability, screenshot, URL/navigation/readiness, focus, lifecycle facets, resource metrics, errors, and postcondition verdict. Semantic-only and screenshot-coordinate-only control are both insufficient.

## Operator takeover and CAPTCHA

Control is an attributable lease. Operator takeover revokes or suspends the model lease, cancels queued input safely, preserves the visible tab, and records the boundary in WP-1 telemetry. A real CAPTCHA or authentication challenge becomes `BLOCKED_OPERATOR`; Stage does not auto-solve or blindly retry it. Deterministic owned fixtures use official reCAPTCHA/hCaptcha test keys.

Background automation must not activate the OS application, steal keyboard focus, or change the active Stage tab unless the action explicitly requires foreground operation.

## Canonical bulk actions

Bulk commands target a frozen snapshot of the canonical Stage registry, never the currently visible, loaded, filtered, grouped, or paginated sidebar rows. Each bulk run records the registry revision, target-list hash, total count, durable target artifact, bounded concurrency, per-target idempotency, resumable cursor, exclusions/protection reasons, and reconciliation:

```text
selected = succeeded + skipped + blocked + failed
```

Concurrent additions are excluded unless a separately named dynamic-set mode is explicitly requested. Post-run verification reads the canonical registry.

## Privacy and request policy

Use one host-owned request-policy pipeline above every engine adapter. The policy order and receipt shape must be deterministic and versioned:

1. normalize URL, source context, destination/resource type, redirect lineage, and session policy;
2. enforce scheme, origin, file, loopback, private-network, and download rules;
3. apply operator allow/deny rules;
4. evaluate content-blocking lists;
5. apply approved response fulfilment or redirect behavior;
6. emit a redacted policy receipt.

Brave's `adblock-rust` is the preferred filtering candidate for the prototype because it is used in Brave, supports network and cosmetic filtering, resource replacement, hosts syntax, uBlock-style syntax, and native/WASM use. Stage must still own list provenance, signed/checksummed updates, compilation rollback, per-profile exceptions, deterministic match diagnostics, and adapter coverage tests. The library does not replace Stage's network-security policy.

## Session and profile contract

- Stage durable records are authoritative; engine profile directories are materialized runtime state.
- Ordinary sessions may share an engine environment only after cookie, HTTP authentication, local/session storage, IndexedDB, cache, service worker, permission, injected-content, and crash-restart isolation tests pass.
- Strong isolation uses a separate engine environment or process and separate storage root.
- Ephemeral means a fresh non-reused directory plus verified cleanup; it does not mean an upstream engine automatically securely deletes state.
- Explicit cookie JSON export remains an operator action. Routine Media Downloader authentication should use a governed scoped credential lease or host-only adapter rather than a reusable whole-profile dump.
- Cross-engine session transfer is explicit, partial, versioned, redacted, auditable, and never implied by a shared `StageSession` ID.
- Browser profile/session files are not Stage's durable source of truth.

## Capture and archive semantics

Stage must distinguish three operations:

| Operation | Source | Intended evidence | Output |
|---|---|---|---|
| Current-renderer capture | The operator's active, authenticated, script-mutated DOM and renderer state. | What the current Stage tab displayed at a specific navigation/generation. | screenshot, DOM/semantic snapshot, selected resources, source metadata, optional MHTML/PDF/Markdown derivative. |
| Independent acquisition/archive | A governed fetch/crawl/archive job outside the current renderer. | Re-fetchable network resources and archival replay. | WARC or archive manifest plus response records and provenance. |
| Selection/media extraction | A selected DOM range, link, image, video, playlist, document, or 3D asset. | Exact selected/source bytes and lineage into downstream workflows. | canonical ArtifactHandle, source hash, media facts, derived-content record, and job receipts. |

SingleFile explicitly does not position itself as professional archival. IIPC WARC 1.1 provides the appropriate independent-archive record model for request/response payloads, metadata, transformations, integrity, and replay-oriented acquisition. Chrome MHTML capture, CDP DOMSnapshot, screenshots, print-to-PDF, and readability extraction are useful current-renderer evidence, not a substitute for independent acquisition. Captured HTML is hostile by default. `trusted HTML` requires an explicit sanitized artifact class, sanitizer version/policy, no active script, constrained resource resolution, and proof that external web content cannot reach the Stage privileged bridge.

## Validation plan

1. Run the project-selected WebDriver BiDi WPT subset against every adapter, plus typed unsupported-capability negatives.
2. Reject stale state revisions, engine generations, locator generations, and navigation IDs.
3. Prove background commands never steal OS or Stage focus.
4. Pair semantic snapshots with geometry, actionability, screenshots, and postconditions.
5. Inject freeze, discard, renderer crash, browser crash, host hard-kill, event loss, timeout, and uncertain-side-effect recovery.
6. Exercise 1, 10, 100, 1,000, and 3,000 durable records while bounding live renderers independently.
7. Run a 3,000-target bulk command with only 50 rendered sidebar rows and reconcile exactly.
8. Measure memory, CPU, process/handle counts, restore/action latency, trace loss, disk growth, and orphan processes.
9. Exercise request policy through redirects, cache, subframes, service workers, authentication, downloads, failures, private/loopback targets, and DNS changes.
10. Prove zero storage leakage across ordinary, strong-isolation, ephemeral, clean-restart, crash, and cleanup paths.
11. Test CAPTCHA fixtures with vendor test keys and external challenges with operator takeover.
12. Correlate every child process and action with WP-1 ownership and telemetry; verify parent-death cleanup.
13. Use WebArena-style reproducible tasks and programmatic postconditions; include BrowserArena-observed CAPTCHA, popup, and direct-navigation failure classes in the Stage-owned corpus.

## Rejected approaches

- CDP, Playwright objects, or engine profile files as Stage authority.
- A single lifecycle state or thousands of eagerly restored renderers.
- Automatic CAPTCHA solving or retry loops.
- Bulk operations based on rendered UI rows.
- Focusing a tab before every action.
- One browser context per tab by default.
- Sharing user-content managers, caches, or storage roots across unproven trust boundaries.
- Treating snapshot formats such as MHTML or SingleFile as professional archive authority.
- Duplicating WP-1 model orchestration inside Stage.

## Primary sources checked

- [WebDriver BiDi specification](https://w3c.github.io/webdriver-bidi/), [explainer](https://github.com/w3c/webdriver-bidi/blob/main/explainer.md), [roadmap](https://github.com/w3c/webdriver-bidi/blob/main/roadmap.md), and [WPT tests](https://github.com/web-platform-tests/wpt/tree/master/webdriver/tests/bidi)
- [Selenium WebDriver BiDi](https://www.selenium.dev/documentation/webdriver/bidi/)
- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/) and [Target domain](https://chromedevtools.github.io/devtools-protocol/tot/Target/)
- [Chrome Page Lifecycle](https://developer.chrome.com/docs/web-platform/page-lifecycle-api)
- [Chromium process model and site isolation](https://chromium.googlesource.com/chromium/src/+/main/docs/process_model_and_site_isolation.md)
- [Playwright browser contexts](https://playwright.dev/docs/next/browser-contexts), [locators](https://playwright.dev/docs/locators), [actionability](https://playwright.dev/docs/actionability), and [ARIA snapshots](https://playwright.dev/docs/aria-snapshots)
- [Servo BiDi draft PR](https://github.com/servo/servo/pull/45266) and [ServoDriver architecture](https://book.servo.org/design-documentation/servodriver.html)
- [reCAPTCHA test keys](https://developers.google.com/recaptcha/docs/faq) and [hCaptcha test keys](https://docs.hcaptcha.com/#integration-testing-test-keys)
- [Firefox Session Store](https://firefox-source-docs.mozilla.org/toolkit/components/sessionstore/)
- [Chromium session restore policy](https://chromium.googlesource.com/chromium/src/+/cc44e4fee54dcf1125de9f0f302aa79b84d4220e/chrome/browser/resource_coordinator/session_restore_policy.h)
- [Brave `adblock-rust`](https://github.com/brave/adblock-rust)
- [SingleFile FAQ](https://github.com/gildas-lormeau/SingleFile/wiki/FAQ)
- [Chrome DevTools DOMSnapshot](https://chromedevtools.github.io/devtools-protocol/tot/DOMSnapshot/)
- [Chrome page capture](https://developer.chrome.com/docs/extensions/reference/api/pageCapture)
- [Playwright authentication state](https://playwright.dev/docs/auth)
- [Google Site Isolation research](https://research.google/pubs/site-isolation-process-separation-for-web-sites-within-the-browser/)
- [WebArena implementation](https://github.com/web-arena-x/webarena)
- [BrowserArena](https://arxiv.org/abs/2510.02418)
- [IIPC WARC 1.1](https://iipc.github.io/warc-specifications/specifications/warc-format/warc-1.1/)

</topic>
