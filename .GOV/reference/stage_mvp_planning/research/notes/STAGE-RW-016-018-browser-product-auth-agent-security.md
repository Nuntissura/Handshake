---
file_id: stage-rw-016-018-browser-product-auth-agent-security
file_kind: research-note
updated_at: "2026-07-19"
status: primary-source-deep-review-complete-validation-pending
wp_id: WP-1-Handshake-Stage-MVP-v1
verification_status: primary-source-deep-review-complete-validation-pending
---

<topic id="stage-rw-016-webview2-browser-product-gap" status="primary-source-deep-review-complete-validation-pending" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# RW-016 WebView2 browser-product gap

## Sources checked

- `STAGE-SRC-WEB-064`: official WebView2 browser-feature differences.
- `STAGE-SRC-WEB-065`: official WebView2 API overview.
- `STAGE-SRC-WEB-066`: Chromium session-history design.
- `STAGE-SRC-WEB-067`: WebView2 download operation/failure behavior.
- `STAGE-SRC-WEB-068`: WebView2 clear-browsing-data API.
- `STAGE-SRC-WEB-086` and `STAGE-SRC-WEB-087`: Firefox Places bookmark/history architecture.
- `STAGE-SRC-WEB-088`: Firefox data-sanitization patterns.
- `STAGE-SRC-WEB-089`: W3C user-agent accessibility guidance.

## Findings

WebView2 is a production embedding runtime, not a complete Edge browser surface. Favorites, profile identity/sync, continue-where-left-off, browser settings/history pages, translation and reader UI, and many browser shortcuts are absent or disabled. Stage must therefore own native omnibox, history, bookmarks, recently closed, restore, settings, translation/readable workflows, browser commands, permissions, downloads, and diagnostic UI.

WebView2 exposes enough host APIs for many of these workflows—cookies/profiles, find, downloads, permissions, popups, context menus, process failures, clear data, screenshots, PDF, history navigation, audio/fullscreen, and CDP—but each becomes Stage product logic, persistence, recovery, and evidence. The API's low-memory target does not stop scripts; suspend/discard/service-worker semantics must be modeled separately.

Chromium's primary design distinguishes per-tab joint session history from profile visit history. Stage needs both and also a third durable recently-closed/window-restore surface.

## Selected approach

Use direct WebView2 as the bounded Windows interactive bootstrap, but build all product/browser-domain behavior above the adapter. Treat unsupported WebView2 browser platform features as explicit capability gaps. Do not introduce a second embedded Chromium wrapper merely to recover missing browser chrome.

## Rejected options

- Relying on Edge internal pages: many are blocked in WebView2 and they would not be Stage-owned or portable.
- Treating a boolean bookmark on a live tab as bookmark authority: bookmarks must outlive tabs and carry independent organization/provenance.
- Treating tab back/forward history as browsing history: the data lifecycles and privacy controls differ.

## Validation

The full browser-action corpus, 3,000-tab import/restore, download interruption, clear-data isolation, and no-context manual journeys must pass on the exact shipped runtime matrix.

</topic>

<topic id="stage-rw-017-authentication-compatibility" status="primary-source-deep-review-complete-validation-pending" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# RW-017 authentication compatibility

## Sources checked

- `STAGE-SRC-WEB-064`: Microsoft explicitly records Google Authentication disabled in embedded WebView2.
- `STAGE-SRC-WEB-069`: Google embedded-WebView OAuth policy.
- `STAGE-SRC-WEB-070`: RFC 8252 external-user-agent OAuth best current practice.
- `STAGE-SRC-WEB-071`: WebView2 CookieManager mutation behavior.
- `STAGE-SRC-WEB-041`: Chrome 136 remote-debugging profile restriction and Chrome-for-Testing recommendation.

## Findings

The primary authenticated YouTube journey conflicts with the selected embedded runtime. External-browser OAuth is the correct native-app flow for API authorization, but it does not create arbitrary website cookies in Stage. Cookie import can create or update compatible profile cookies, but it is a high-sensitivity, partial, provider-dependent transfer rather than a universal login mechanism. Remote control of an operator's default Chrome profile is rejected; current Chrome requires a non-default data directory for remote debugging and recommends Chrome-for-Testing for automation.

## Selected approach

Make authentication mode and compatibility an explicit per-journey result. Support secure engine-native login where allowed, RFC 8252 external OAuth for API-backed workflows, governed operator-driven session import where compatible, and a separately gated full-browser compatibility route if later proven necessary. Otherwise return `UNSUPPORTED` or `SECURITY_BLOCKED` without pretending the primary workflow passed.

## Validation

YouTube anonymous, embedded sign-in failure, supported API OAuth, session import, restored session, expiry, logout, Downloader handoff, MFA, WebAuthn, HTTP auth, proxy auth, and client-certificate fixtures are required before the Windows production bootstrap gate can pass.

</topic>

<topic id="stage-rw-018-browser-agent-security" status="primary-source-deep-review-complete-validation-pending" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# RW-018 browser-agent security

## Sources checked

- `STAGE-SRC-WEB-072`: OpenAI browser-agent prompt-injection engineering guidance.
- `STAGE-SRC-WEB-073`: OpenAI URL/data-exfiltration controls.
- `STAGE-SRC-WEB-074`: OpenAI prompt-injection user/control model.
- `STAGE-SRC-WEB-075`: NIST AI agent-evaluation guidance using prompt-injection benchmarks.

## Findings

Prompt injection is a long-lived browser-agent security problem. Input classification alone does not reliably catch sophisticated social-engineering attacks. Defenses must constrain capabilities and data flows even when a model is manipulated. URL fetches, navigations, images, forms, uploads, and tool calls are data sinks; untrusted page content can attempt to exfiltrate secrets through any of them.

## Selected approach

Treat page content as non-authoritative data, enforce source-to-sink and consequence policy before dispatch, use least-privilege sessions/capabilities, durable pre-side-effect intents, confirmation/watch/takeover for risky actions, stale-target rejection, uncertain-outcome reconciliation, and continuous adversarial evaluation. Model training/detection is one layer, not the acceptance surface.

## Validation

The release corpus must attempt cross-origin exfiltration, privilege expansion, secret access, stale target use, confirmation manipulation, persistence into later context, and repeated non-idempotent effects across DOM, accessibility, screenshots/OCR, PDFs, frames, downloads, service workers, captures, and search results.

</topic>
