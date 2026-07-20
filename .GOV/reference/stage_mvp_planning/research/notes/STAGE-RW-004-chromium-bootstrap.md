---
file_id: stage-rw-004-chromium-bootstrap
file_kind: reference-research-note
updated_at: "2026-07-19"
status: primary-source-deep-review-complete-validation-pending
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-rw-004-chromium-bootstrap" status="primary-source-deep-review-complete-validation-pending" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# STAGE-RW-004: Chromium bootstrap

## Decision supported by the research

Use direct WebView2 through `webview2-com` as the Windows-first embedded Chromium bootstrap, isolated behind the Stage-owned browser-engine adapter. Start with a child-HWND browser island in the native egui/winit shell. Run a bounded `ICoreWebView2CompositionController` spike only if child-window clipping, overlay, transform, or z-order behavior fails.

Use Wry only for a disposable ergonomic prototype. Keep a pinned Chrome-for-Testing worker as an optional deterministic headless-validation lane. Escalate to CEF through `tauri-apps/cef-rs` only if the native integration requires genuine offscreen rendering, same-Chromium cross-platform behavior, or embedding capabilities that WebView2 cannot supply.

This is a bootstrap decision, not a reversal of the operator-locked Servo direction. Chromium-specific types and behavior must remain below the adapter boundary, and Chromium bootstrap acceptance cannot satisfy the whole Stage WP.

## Candidate comparison

| Candidate | Evidence-backed strengths | Evidence-backed constraints | Disposition |
|---|---|---|---|
| Direct WebView2 | First-party Windows Chromium embedding; multi-process Chromium model; profiles and cookies; request observation/mutation/block/fulfil; CDP access; process-failure events; Evergreen servicing. | `webview2-com` is a mostly unsafe Rust COM projection; no supported true headless/offscreen renderer; app must manage UDF/profile/process lifetime and runtime restart. | SELECTED for Windows bootstrap. |
| Wry over WebView2 | Rust ergonomics, child webviews, shared contexts, incognito, IPC, custom protocols, devtools. | Stable abstraction does not expose arbitrary HTTP(S) interception; cross-platform system webviews do not provide behavioral parity; Wry types would make later replacement harder. | Prototype only. |
| External Chrome-for-Testing | Versioned automation build; WebDriver/BiDi; CDP Fetch/Target/Storage/Network/capture; strong deterministic headless lane. | No native egui embedding; separate profile/process/endpoint security; pinned distribution and patch ownership; CDP compatibility churn. | Optional worker, never primary interactive surface. |
| CEF plus `cef-rs` | Embedding-oriented multi-process Chromium; request handlers; RequestContexts; remote debugging; real offscreen rendering; Windows/Linux/macOS. | Large binary and packaging surface; message-loop/FFI/sandbox-helper work; application-owned Chromium updates; offscreen resize/compositing complexity. | Conditional fallback only. |
| Servo | Strategic Rust-native engine target. | Current upstream describes it as a prototype; embedding remains active work; current Windows content processes are unsandboxed. | Future strategic adapter; rejected as this month's arbitrary-web bootstrap. |

## Selected bootstrap topology

```text
Stage native shell
  -> Stage browser-engine adapter
       -> WebView2 embedded controller (interactive Windows tabs)
       -> Chrome-for-Testing worker (optional headless validation)
       -> CEF adapter (only after an evidence-backed escalation)
       -> Servo adapter (strategic implementation and restricted alpha)
```

The adapter exposes Stage concepts rather than COM, HWND, CDP, Wry, CEF, or Servo types:

- surface attachment, bounds, visibility, focus policy, and close;
- persistent and ephemeral profiles, cookie operations, data clearing, and storage policy;
- navigation, script evaluation, structured snapshot, screenshot, input, and events;
- network inspect, block, redirect, fulfil, and download observation;
- runtime version, process identities, failure events, restart, and capability discovery.

Unsupported behavior returns a typed `UNSUPPORTED_CAPABILITY`. The host never silently selects another engine.

## WebView2 technical implications

- Use profiles in one user-data folder for ordinary isolation only after storage-surface leakage tests pass. A separate environment/UDF is required when stronger process, fault, or retention isolation is needed.
- Treat `ProcessFailed` and `BrowserProcessExited` as normal recoverable lifecycle events. The Stage record remains durable while runtime attachment changes.
- Prefer Evergreen for the bootstrap to avoid shipping a full fixed Chromium runtime, but record the runtime version and expose a state-preserving restart path when a newer runtime is available.
- Do not call CompositionController "offscreen". It joins the application's DirectComposition tree, and the host must forward input.
- Verify request-policy coverage for main frames, subframes, redirects, cache, service workers, authentication, downloads, response replacement, and failure paths before claiming ad blocking or network policy works.

## External worker controls

Chrome 136 and later require a non-default profile for remote debugging. Any Chrome-for-Testing worker must therefore use an isolated per-task profile, a pipe or randomized loopback endpoint, sandboxing enabled, version negotiation, bounded lifetime, and WP-1 process-ownership/reap records. Raw CDP stays private to the adapter. Standardized WebDriver BiDi is preferred when it covers the operation.

## Required prototype spike

1. Child WebView2: resize, DPI, multiple monitors, focus, keyboard traversal, IME, clipboard, drag/drop, context menus, popups, downloads, accessibility, device loss, and clean shutdown.
2. Native-shell composition: clipping and overlays; test CompositionController only where the child controller fails.
3. Sessions: two persistent profiles and one ephemeral profile across cookies, localStorage, IndexedDB, cache, permissions, and service workers.
4. Network policy: block, redirect, fulfil, authentication, subframes, cached traffic, service workers, downloads, and errors.
5. Automation/diagnostics: DOM and accessibility snapshots, console/network traces, screenshots, script evaluation, process correlation, and crash recovery.

Decision ladder:

- Child controller passes: ship it.
- Child fails only composition needs and CompositionController passes: ship composition.
- True texture/offscreen output is required: spike CEF OSR.
- Deterministic background automation exceeds embedded WebView2: add the pinned Chrome-for-Testing worker.

## Promotion and retirement gates

- No focus, IME, accessibility, clipping, or z-order defects across repeated DPI/multi-monitor cycles.
- No cross-profile leakage across every supported storage surface and crash/restart path.
- Network-policy coverage proven across all request paths listed above.
- Structured snapshot, screenshot, console/network trace, and evaluation proven against a real Stage tab.
- Renderer, GPU, and browser termination leave the host alive and restore the intended Stage record.
- Runtime, process, profile/UDF, adapter version, and capability manifest are observable.
- Representative scale is measured on target Windows 10/11 x64 and ARM64 hardware; no thresholds are invented before measurement.
- Clean-machine packaging covers missing-runtime, offline, signing, SBOM, notices, rollback, and actual payload.
- Fake-backend and WebView2 conformance suites pass; vendor types do not cross the adapter boundary.

## Rejected options

- Wry as the durable Stage architecture.
- Raw CDP as the public or canonical Stage protocol.
- Remote debugging against the operator's default browser profile.
- `--no-sandbox` for any Chromium worker.
- CompositionController described or accepted as true offscreen rendering.
- CEF adoption before a failed WebView2 integration spike demonstrates its need.
- Bootstrap feature expansion that creates a second co-equal browser product.

## Primary sources checked

- [WebView2 CompositionController](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2compositioncontroller)
- [WebView2 web-resource requests](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/webresourcerequested)
- [WebView2 multi-profile support](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/multi-profile-support)
- [WebView2 user-data folders](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder)
- [WebView2 process model](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model)
- [WebView2 security measures](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/measures)
- [WebView2 process-related events](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-related-events)
- [WebView2 distribution](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)
- [`webview2-com`](https://docs.rs/crate/webview2-com/latest)
- [Wry WebViewBuilder](https://docs.rs/wry/latest/wry/struct.WebViewBuilder.html)
- [Wry HTTP interception limitation](https://github.com/tauri-apps/wry/issues/1087)
- [ChromeDriver](https://developer.chrome.com/docs/chromedriver)
- [Chrome remote-debugging change](https://developer.chrome.com/blog/remote-debugging-port)
- [Chrome for Testing rationale](https://developer.chrome.com/blog/tools-from-chrome-for-frictionless-testing)
- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
- [CEF general usage](https://chromiumembedded.github.io/cef/general_usage.html)
- [`cef-rs`](https://github.com/tauri-apps/cef-rs)
- [CEF OSR resize issue](https://github.com/chromiumembedded/cef/issues/3826)

</topic>
