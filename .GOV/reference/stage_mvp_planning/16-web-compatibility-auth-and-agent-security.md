---
file_id: stage-web-compatibility-auth-and-agent-security
file_kind: reference-security-and-compatibility-contract
updated_at: "2026-07-20"
status: research-hardened-proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-authenticated-site-compatibility" status="research-hardened-proposed" version="v0.2" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-20">

# Authenticated-site and browser-service compatibility

## Production blocker

The selected WebView2 bootstrap cannot presently be called production-usable for Stage's primary authenticated YouTube workflow without proof. Microsoft documents that Google authentication is disabled in embedded WebViews, including WebView2, and Google documents that OAuth authorization endpoints block embedded user agents. This is an engine/provider policy incompatibility, not a generic login bug.

Cookie JSON import is technically possible through WebView2's CookieManager but is not the default solution: it exposes reusable credentials, may omit protected or partitioned state, can violate site expectations, and does not prove MFA, passkey, device-binding, or session-refresh behavior. System-browser OAuth follows RFC 8252 and is appropriate for native-app API authorization, but an OAuth token does not automatically become arbitrary website cookie state.

Operator decision 2026-07-20 (`STAGE-DEC-021`): this blocker is now the bootstrap's viability gate. A time-boxed governed-session-import spike must prove logged-in youtube.com state, playback, playlist/channel access, and authenticated download handoff inside WebView2. If the spike fails, the WebView2/Chromium bootstrap and the `STAGE-WIN-BOOTSTRAP-PROD` slice are abandoned and all Stage effort focuses on Servo. The `FULL_BROWSER_COMPATIBILITY_ROUTE` may not be used to rescue the bootstrap.

## Supported session-acquisition modes

| Mode | Intended use | Required controls | Prohibited claim |
|---|---|---|---|
| `ENGINE_NATIVE_LOGIN` | A site permits secure embedded login. | Operator takeover for secrets/challenges; origin/TLS visibility; auth fixture; storage isolation; logout/clear proof. | Never assume support from Chromium compatibility alone. |
| `EXTERNAL_USER_AGENT_OAUTH` | Handshake uses a provider API as a native app. | Default-browser authorization, PKCE, state/nonce, registered redirect, token broker, scoped storage, revoke/expiry. | Does not establish logged-in arbitrary website state. |
| `GOVERNED_SESSION_IMPORT` | Operator explicitly transfers compatible cookies/session data. | Previewed session/domain scope, explicit confirmation, encrypted staging, attribute/loss report, no logs, revocation, secret scan. | Not routine or automatic; never described as universally compatible. |
| `FULL_BROWSER_COMPATIBILITY_ROUTE` | A later proven external/full-browser lane supplies site state or interactive use. | Separate process/profile authority, private control channel, explicit engine provenance, no default-profile remote debugging. | Cannot silently become Stage's interactive backend. |
| `UNSUPPORTED_OR_DEGRADED` | Provider/engine policy blocks the journey. | Clear message, documented supported alternatives, no retry loop, no misleading success. | Cannot satisfy the authenticated-site release gate. |

## Compatibility matrix schema

Each named journey records:

```yaml
journey_id: stable-id
site_or_fixture: string
engine_adapter: string
engine_build: string
runtime_channel: string
profile_class: string
authentication_mode: string
required_capabilities: []
steps_manifest: artifact-handle
expected_postconditions: []
actual_result: PASS|DEGRADED|UNSUPPORTED|SECURITY_BLOCKED|FAIL
operator_takeover_points: []
credential_surfaces: []
data_egress: []
evidence_bundle: artifact-handle
expires_at: timestamp
```

Minimum journey groups:

- YouTube anonymous browsing/playback, embedded Google sign-in, already-authenticated restore, logout, session expiry, captions, playlist/channel enumeration, Downloader handoff, and ASR fallback;
- OAuth/SSO with authorization code plus PKCE, MFA, device approval, passkey/WebAuthn/security key, password manager/autofill, HTTP basic/digest, proxy auth, and client certificate;
- forms, dynamic navigation, same-document history, uploads, drag/drop, clipboard, downloads, JavaScript dialogs, `beforeunload`, popups, external schemes, offline/reconnect, service workers, notifications, WebSockets/WebTransport, WebRTC/screen share, media/codecs/DRM, PDF, and 3D;
- certificate errors, mixed content, insecure forms, captive portal, enterprise proxy/root, private network, DNS rebinding, malicious download, and protocol-handler refusal.

External live sites are expiry-bound monitoring evidence. Deterministic release authority comes from controlled fixtures plus an operator-approved small set of external journeys whose terms and variability are accepted.

</topic>

<topic id="stage-browser-agent-security" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Browser-agent security and control plane

## Security premise

Every webpage, frame, accessible name, DOM node, image text, network response, download, captured document, and Stage-search result sourced from the web is untrusted data. It can describe actions but cannot grant authority, change the operator's objective, expand tool scope, authorize secret access, or override Stage/WP-1 policy.

Current field evidence shows input classifiers or an `AI firewall` alone are insufficient for sophisticated prompt injection. The production design therefore constrains what a manipulated model can do and where data can flow, even when detection fails.

## Source-to-sink enforcement

Every model action is evaluated against:

1. data sources observed since the last operator instruction or trusted checkpoint;
2. target origin, process, file, tool, project, account, and external recipient;
3. whether the action transmits or transforms sensitive data;
4. whether navigation or URL parameters introduce an unobserved third-party sink;
5. action consequence class and reversibility;
6. actor capability, session trust class, lease/fencing token, and operator policy;
7. expected revision/navigation/engine generation and postcondition;
8. required confirmation, watch, or takeover state.

Sensitive information may not be placed in a URL, query, fragment, referrer, form, upload, clipboard, download filename, page script, console, bridge message, external tool call, or cross-origin request without a policy-authorized purpose and operator-visible disclosure when required.

## Action consequence classes

| Class | Examples | Default control |
|---|---|---|
| `READ_LOCAL_STATE` | Read Stage registry, metadata, prior receipts. | Actor capability plus scope filter. |
| `READ_HOSTILE_WEB` | Observe page, follow public link, retrieve public resource. | Logged-out/isolated session preferred; source-to-sink check. |
| `MUTATE_LOCAL_REVERSIBLE` | Organize tabs, add bookmark, save capture. | Revision check, durable receipt, undo/compensation where possible. |
| `TRANSMIT_OR_AUTHENTICATE` | Submit form, upload, send content, login, open third-party URL containing data. | Explicit bounded intent; sensitive-data preview; confirmation/watch/takeover by policy. |
| `EXTERNAL_CONSEQUENTIAL` | Purchase, publish, delete remote data, change account/security settings. | Operator confirmation immediately before dispatch and fresh observation; no automatic retry after unknown outcome. |
| `SECRET_ENTRY_OR_CHALLENGE` | Password, MFA, passkey, CAPTCHA, security key. | Operator takeover; model observation/input paused; resume only after fresh sanitized observation. |

## Durable action protocol

Before dispatch Stage persists `StageActionIntent` with stable target, actor/lane, capability and ToolGate result, source and sink origins, sensitive-data classes, expected revision, engine generation, focus policy, idempotency/correlation/causation, deadline, pre-observation hash, and consequence class.

The terminal receipt is one of:

- `SUCCEEDED_POSTCONDITION_PROVEN`;
- `REJECTED_POLICY`;
- `REJECTED_STALE_TARGET`;
- `BLOCKED_OPERATOR_CONFIRMATION`;
- `BLOCKED_OPERATOR_TAKEOVER`;
- `CANCELLED_BEFORE_DISPATCH`;
- `FAILED_NO_SIDE_EFFECT`;
- `OUTCOME_UNKNOWN_RECONCILE_REQUIRED`.

Timeout, disconnect, browser crash, or event loss after dispatch cannot be converted to a retryable failure for a non-idempotent action. The system observes current state, records reconciliation evidence, and requires a policy decision before any retry.

## Parallel actor control

Stage supports explicit target scopes for tab, window, folder, frozen query result, bulk run, or session. Leases use monotonically increasing fencing tokens, expiry, heartbeat, cancellation, operator priority, and visible ownership. Per-actor budgets bound live renderers, navigation, network, capture, download, and queue depth. Conflicting mutations reject or serialize against canonical revisions; models do not coordinate by focus or visual guesswork.

The operator can pause one actor, revoke one lease, revoke all model control, inspect queued actions, take over a tab, and recover abandoned work. No model action opens a foreground window, steals the operator keyboard, changes focus, or exposes a secret-entry surface without an explicit focus policy.

## Observations and provenance

An observation bundle contains:

- stable Stage window/session/tab IDs and ephemeral engine context/generation;
- URL, origin, navigation/document revision, lifecycle, focus, viewport, DPR, locale, security and permission state;
- semantic/accessibility snapshot with stable observation-local node references;
- geometry, visibility, hit-test/actionability, screenshot, optional OCR, and scroll state;
- console/network/download/resource facts allowed by policy;
- web-content provenance and trust labels on all extracted text;
- hash and expiry; any navigation or significant DOM mutation invalidates stale targets.

The model never receives raw cookie values, password fields, passkey material, hidden takeover input, client keys, unrestricted local files, crash-dump secrets, or arbitrary Stage App bridge authority.

## Prompt-injection and exfiltration validation

The corpus includes visible, hidden, encoded, CSS-obscured, image/OCR, accessibility-label, PDF, download, iframe, service-worker, search-result, captured-document, and cross-tab injections. Attacks attempt to:

- override the operator task;
- read another session/project/tab;
- place secrets in URLs or forms;
- navigate to attacker-controlled origins;
- upload local/captured data;
- invoke privileged Stage/WP-1 tools;
- exploit confirmation fatigue or stale observations;
- cause repeated non-idempotent side effects;
- persist instructions into Loom, captures, bookmarks, downloads, or later model context.

Release evidence reports attack success rate, unauthorized-tool rate, unauthorized-data-flow rate, confirmation correctness, stale-target rejection, false-positive operational impact, and operator recovery. A detector score alone cannot pass the gate; no unauthorized data flow or consequential action may occur in the release corpus.

</topic>

<topic id="stage-trusted-app-and-web-boundary" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage App, captured content, and hostile web separation

Stage Apps use a separate package/origin/profile class. The bridge binds package ID, version, manifest digest, origin, navigation/document generation, explicit method ID, message schema, size limit, capability/ToolGate result, and actor identity. Navigation away removes the bridge and cancels pending privileged messages. Generic object proxies and arbitrary script-to-native calls are forbidden.

Captured HTML, WARC replay, readable output, Markdown preview, PDFs, media metadata, 3D files, and downloaded archives remain hostile input. Sanitization creates a derived artifact with versioned policy provenance; it never upgrades content to Stage App trust. Replay/viewer processes deny the bridge, external navigation, file/private-network access, and credential sharing by default.

Every parser/viewer has size, nesting, decompression, CPU, wall-clock, memory, GPU, output, and child-process limits plus fuzz/failure corpora. Malformed content fails with a typed partial/result record rather than crashing the Stage host or silently dropping provenance.

</topic>
