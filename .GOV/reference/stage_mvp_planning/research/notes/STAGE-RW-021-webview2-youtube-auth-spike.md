---
file_id: stage-rw-021-webview2-youtube-auth-spike
file_kind: reference-research-note
updated_at: "2026-07-20"
research_workstream: STAGE-RW-021
verification_status: harness-built-and-run-authenticated-youtube-session-import-PASS-durability-open
---

<topic id="stage-rw-021-webview2-youtube-auth-spike" status="hardened" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-20">

# WebView2 authenticated-YouTube viability spike (STAGE-DEC-021)

## Question

Can the WebView2/Chromium bootstrap deliver authenticated YouTube through governed session import (cookie injection), or must Stage abandon the Chromium bootstrap and focus on Servo? This is the `STAGE-DEC-021` viability gate.

## What was built and run

A Rust WebView2 harness was built and executed on this machine (Windows 11, WebView2 runtime 150.0.4078.83, Rust 1.91.1). It embeds an Edge WebView2 controller in a host-owned Win32 window — the exact architecture Stage would use — optionally injects exported cookies via `ICoreWebView2CookieManager`, navigates to a target URL, probes the DOM for logged-in signals, captures a screenshot, and writes a machine-readable verdict.

The spike code, dependencies, and runtime live entirely outside the governance kernel, on the C: disk, per operator instruction:

- Project: `C:\Handshake_Stage_Spike\webview2-youtube-auth-spike\`
- Build target: `C:\Handshake_Stage_Spike\cargo-target\`
- WebView2 profile: `C:\Handshake_Stage_Spike\wv2-profile\`
- Outputs: `C:\Handshake_Stage_Spike\out\` (and `out-synthetic\`)
- Operator run guide: `C:\Handshake_Stage_Spike\README.md`

## Results (autonomously verified, with the operator's real session)

Cookie source: the operator's logged-in Google/YouTube session lives in the Firefox `default-release` profile (`rrnwzxin.default-release`) — 1528 cookies, all 15 Google auth cookies present. Firefox stores cookie values unencrypted, so they were read directly from `cookies.sqlite` and exported to the harness JSON format. (The app the operator named, VoxVulgi at `C:\Program Files\VoxVulgi`, is linked to Firefox; its own profile was empty. The live session is in the standard Firefox release profile.)

Run 1 — anonymous (no cookies): `navigation_completed: true`, `title: "YouTube"`, `has_player: true`, `logged_in_ytcfg: false`, `has_signin: true`. Real 1280x900 PNG (~107 KB). WebView2 embeds and renders YouTube; login-probe correctly reports signed-out.

Run 2 — synthetic cookies: `cookies_injected: 2`, no error, `verdict: COOKIES_REJECTED_SIGNED_OUT`. The `ICoreWebView2CookieManager::AddOrUpdateCookie` injection path works.

Run 3 — full real cookie set (319 google+youtube cookies): `cookies_injected: 319`, but YouTube returned `HTTP 413 Request Entity Too Large`. Injecting every Google-property cookie made the request Cookie header exceed the server limit. This is a concrete design constraint, not a failure: session import must scope cookies to the target site's essential set.

Run 4 — essential YouTube auth cookies (40 named `.youtube.com` cookies): **`verdict: AUTHENTICATED_STATE_OBSERVED`.** `logged_in_ytcfg: true`, `has_avatar: true`, `has_signin: false`, `has_player: true`, `title: "(1092) YouTube"` (the notification badge only renders when logged in).

Run 5 — auth-gated page (`/feed/subscriptions`, requires login): `title: "(1092) Subscriptions - YouTube"`, `logged_in_ytcfg: true`, `has_avatar: true`, no redirect to a login wall. Screenshot visually inspected: fully logged-in YouTube Premium UI (account avatar, "9+" notification badge, Create button, personalized subscriptions feed). Definitive proof the imported session is genuinely usable, not cosmetic.

## Verdict and direction (empirical — corrects the earlier field-evidence prediction)

- Mechanism: PROVEN.
- `STAGE-DEC-021` gate, immediate/short-term:
  - Part 1 (logged-in state): **PASS** (empirically observed).
  - Part 2 (playback surface present): **PASS**.
  - Part 3 (auth-gated playlist/channel/feed): **PASS** (subscriptions feed rendered logged-in).
  - Part 4 (authenticated download handoff): NOT TESTED (needs Media Downloader credential lease; out of harness scope).
- Governed session import into WebView2 works for authenticated YouTube. This corrects the earlier prediction of FAIL/FRAGILE. The key distinction the field evidence obscured: Google's embedded-webview block targets INTERACTIVE login (the accounts.google.com login flow refuses an embedded user agent). It does NOT block presenting already-valid session cookies — session import sidesteps the login flow entirely, so it is not blocked. DBSC threatens durability over time, not the initial import; this account either is not yet DBSC-enforced or falls back to long-lived cookies.
- Two hard engineering constraints for the real Stage feature, surfaced by the spike:
  1. Cookie scoping is mandatory. Importing the full cross-property Google cookie set triggers HTTP 413. Stage's session-import must select the target site's essential auth cookies (for YouTube, ~40 `.youtube.com` cookies), not dump the whole jar.
  2. Firefox is the practical session source on this machine (unencrypted `cookies.sqlite`); a Chrome/Edge source would require OS-level DPAPI decryption of cookie values.
- Remaining open question — DURABILITY: does the imported session survive over hours/days as YouTube rotates cookies under DBSC/session-refresh? Not yet soak-tested. This is the one thing that could still push toward a system-browser/OAuth session-handoff design, but it does not block the WebView2 bootstrap now.

## Bootstrap decision impact

`STAGE-DEC-021` does NOT fire on this evidence: authenticated YouTube via governed session import is empirically viable in WebView2, so the `STAGE-WIN-BOOTSTRAP-PROD` slice stays alive. The production hardening question narrows from "can it authenticate at all" to "how durable is an imported session, and what is the re-import UX when it lapses." Servo planning inherits the same session-import approach and the same durability question.

## Remaining operator checks (optional, to close durability)

1. Durability soak: re-run against `C:\Handshake_Stage_Spike\my-cookies.json` after several hours; a session that flips to signed-out indicates DBSC/refresh lapse and the need for periodic re-import.
2. Part 4: wire an authenticated download through the Media Downloader credential-lease path once that exists.

## Sources checked

- STAGE-SRC-WEB-100: MicrosoftEdge/WebView2Feedback issues #1578, #2552, #1584 — Google auth "This browser or app may not be secure" in WebView2.
- STAGE-SRC-WEB-101: Google security blog, "Protecting cookies with Device Bound Session Credentials" — exported cookies become useless off-device.
- STAGE-SRC-WEB-102: Google Workspace Updates, May 2026 — DBSC generally available in Chrome for Windows.
- STAGE-SRC-WEB-103: Chrome for Developers, "Device Bound Session Credentials" — TPM-bound non-exportable key, graceful fallback.

</topic>
