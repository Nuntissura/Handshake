---
file_id: stage-rw-001-003-009-servo-production-readiness
file_kind: reference-research-note
updated_at: "2026-07-19"
research_workstreams:
  - STAGE-RW-001
  - STAGE-RW-002
  - STAGE-RW-003
  - STAGE-RW-009
verification_status: primary-source-deep-review-complete-validation-pending
---

<topic id="stage-servo-production-readiness" status="research-complete-validation-pending" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Servo production-readiness research

## Evidence-backed verdict

Servo remains the operator-locked strategic/default renderer direction, but it is not ready for arbitrary-web production on Windows as of 2026-07-19. Current Servo source explicitly launches Windows content processes unsandboxed. A Windows binary and multiprocess support do not satisfy the missing OS-sandbox boundary.

A defensible July rollout is therefore split:

- Servo restricted alpha: trusted or tightly allowlisted content only, after the embedding, isolation, cleanup, diagnostics, and workload gates in this note pass.
- Chromium bootstrap: arbitrary-web first-usable path, behind the same Stage kernel and an explicit adapter capability manifest.
- Servo arbitrary-web promotion: blocked until effective Windows content-process sandboxing and the complete security/compatibility gates pass. Calendar timing cannot waive this gate.

## Verified current state

| Area | Verified evidence | Stage consequence |
|---|---|---|
| Releases | Latest regular release is `v0.3.0` (2026-06-25); `v0.1.2` LTS shipped 2026-07-06. | Pin tag plus full commit; distinguish regular and LTS branches. |
| API version drift | Live Servo crate documentation reports a development version ahead of the published regular release. | Generate and archive API docs from the exact pinned commit; do not compile production contracts from live docs. |
| Embedding lifecycle | Servo embedding requires host-owned event-loop wakeup, builder/configuration, rendering context, WebView delegates, input/event forwarding, and repeated event-loop spinning. | Build a real owner-thread adapter with queues and backpressure; do not treat Servo as a drop-in URL widget. |
| Thread ownership | `WebView` is `!Send` and `!Sync`; the embedder owns repaint/present behavior. | All engine objects stay on one owner event-loop thread; WP1, Stage jobs, and UI communicate by typed messages/receipts. |
| Instance/config isolation | All WebViews in one Servo instance share one Constellation; important `Opts` configuration is global/one-time while some preferences are mutable. | Treat hard trust/profile boundaries as separate Servo instances and preferably separate OS processes until isolation is proven. |
| Windows multiprocess | Windows multiprocess support exists. | Useful reliability separation, but not a security boundary without a sandbox. |
| Windows sandbox | Current `components/constellation/sandboxing.rs` reports sandboxed multiprocess unsupported on Windows and starts an `unsandboxed child process`. | Arbitrary remote content is a release blocker on Windows. |
| Windows upstream CI | Upstream workflow builds, smoke-tests, and packages Windows; it does not constitute Stage-specific Windows WPT, sandbox, multiprocess, soak, or workflow proof. | Stage owns target-specific security, compatibility, reliability, and performance CI. |
| LTS | LTS is best-effort, security-fixes-only, expected nine-month support, no fixed patch schedule, no security guarantee, and excludes servoshell. | Maintain a Stage patch/update/rollback policy and security-response ownership; LTS is not an SLA. |
| Release compatibility | Servo documents breaking monthly API changes while release analysis/stability matures. | Use a narrow adapter, dependency lock, qualification suite, and tested rollback. |
| Storage | Temporary storage creates a unique directory but currently leaves it after Servo exits. | Isolation is not deletion; Stage must own cleanup, crash-recovery cleanup, receipts, and retention policy. |
| Web compatibility | Official 2026-07-19 WPT data is useful trend evidence but covers enabled tests on a development revision, not the pinned release's Windows Stage workload. IndexedDB/CSP/WebDriver remain materially below total conformance. | Pin Servo and WPT revisions; track disabled tests/flakes; require a Stage-owned compatibility corpus. |
| Accessibility/model control | Servo's accessibility tree is still described as basic/experimental, and Servo BiDi work is not yet a production contract. | Pair semantic data with DOM/geometry/screenshot fallbacks; keep model protocol adapter capability-gated. |
| Request policy | Current embedding request interception exposes destination and referrer data useful for ad blocking. | Evaluate one host policy pipeline with `adblock-rust`; verify every resource class and redirect. |
| Crash reporting | Upstream issue tracking documents missing external crash-reporting/minidump support. | Stage requires its own watchdog, dumps, correlation, restart budget, orphan cleanup, and operator-visible disable/rollback path. |
| Performance | Current issues report incomplete Speedometer runs and subsequent navigation failures. | Do not use one benchmark as the gate; run Stage journeys and post-benchmark health checks. |
| Verso | Verso is archived and says it could not keep pace with Servo revisions. | Use only as historical design evidence; reject it as the product base. |

## Selected architecture

1. Pin Servo by release tag, full commit, Cargo lockfile, Rust toolchain, enabled features, Windows SDK/MSVC versions, and generated API documentation.
2. Keep a small `BrowserEngineAdapter`/Servo adapter around a dedicated engine owner thread.
3. Keep Stage windows/tabs/sessions durable and independent from Servo WebView/process IDs.
4. Use separate Servo instances/processes for hard trust or profile boundaries until same-instance isolation is positively proven.
5. Treat interactive, restricted, and headless operation as capability profiles over one adapter, not separate products and not silent fallback paths.
6. Record the exact adapter, engine version/commit, process generation, storage root, capability manifest, and security posture on every session, observation, action, capture, and diagnostic receipt.
7. Keep downstream patches minimal, upstream-first, isolated, rebased, and covered by patch-specific tests. Maintain a known-good rollback pin and feature kill switch.

## Required promotion gates

### Dependency and supply chain

- Reproducible pinned build plus generated SBOM and license inventory.
- Advisory and transitive-dependency scan.
- Documented update intake, compatibility qualification, rollback, and emergency-disable path.
- No production dependency on live unversioned docs or main-branch behavior.

### Embedding contract

- Navigation/history/reload/error pages.
- Paint/present, resize, DPI, focus, keyboard/IME, pointer/touch, clipboard policy.
- Dialogs, HTTP auth, permissions, downloads, popups, request interception, proxy/certificate failures.
- Graceful close, forced renderer death, browser/host hard-kill, restart, and orphan cleanup.
- Queue saturation, cancellation, deadline, and owner-thread responsiveness.

### Profile and storage isolation

Prove zero cross-profile leakage for cookies, HTTP authentication, local/session storage, IndexedDB, HTTP cache, permissions, history-equivalent state, and injected content across simultaneous WebViews, separate instances, restart, crash restart, storage-root reuse, and cleanup.

### Windows security

Arbitrary-web promotion requires an effective default-deny Windows content-process sandbox, independently tested for escape and host/resource access. Until then:

- only trusted/allowlisted sources;
- no valuable credentials;
- strict outbound/private-network/file/internal-scheme policy;
- no shared user-content managers or storage roots across trust boundaries;
- prominent renderer-security posture and a kill switch.

### Compatibility

- Pin Servo and WPT revisions.
- Run relevant WPT suites on Windows, including disabled-test and flake accounting.
- Permit no unexplained regression from the accepted baseline.
- Require 100% pass on a smaller Stage journey corpus covering target research/auth/forms/media/download/capture sites and local test fixtures.
- Re-run the corpus for every Servo upgrade and patch-queue change.

### Reliability, diagnostics, and performance

- Repeated navigation/reload/close and 1/10/100/live-renderer workload cycles.
- 1/10/100/1,000/3,000 durable-tab fixtures with bounded renderer count.
- Crash injection, malformed content, memory pressure, GPU reset, sleep/resume, offline/proxy/certificate failures.
- Post-benchmark navigation and shutdown checks.
- Structured diagnostics with engine version/commit, Stage IDs, process IDs, navigation phase, crash/hang reason, dump reference, cleanup result, and restart count.
- Operator-approved resource thresholds derived from target-hardware baselines, not invented constants.

## Rejected assumptions and options

- Windows binaries imply Windows sandboxing: rejected.
- Multiprocess implies safe arbitrary-web isolation: rejected.
- Green upstream Windows build implies Stage compatibility/security: rejected.
- Live docs describe the pinned release: rejected.
- LTS provides a security SLA: rejected.
- WPT aggregate percentages alone prove Stage readiness: rejected.
- Temporary storage means secure deletion: rejected.
- Shared Servo instance/config/content manager is safe for hard tenant separation: unverified and rejected as a default.
- Verso is a maintained embedding base: rejected.
- One historical penetration test validates the full embedded-browser threat surface: rejected.

## Sources checked

- Servo releases: https://github.com/servo/servo/releases
- Servo v0.3.0: https://github.com/servo/servo/releases/tag/v0.3.0
- Servo LTS policy: https://book.servo.org/embedding/lts-release.html
- Servo embedding overview/API: https://book.servo.org/embedding/overview.html and https://doc.servo.org/servo/
- Servo architecture: https://book.servo.org/design-documentation/architecture.html
- Current sandbox source: https://github.com/servo/servo/blob/main/components/constellation/sandboxing.rs
- Current Windows CI: https://github.com/servo/servo/blob/main/.github/workflows/windows.yml
- April 2026 Servo update: https://servo.org/blog/2026/05/31/april-in-servo/
- May 2026 Servo update: https://servo.org/blog/2026/06/30/may-in-servo/
- Official WPT score data: https://wpt.servo.org/scores.json
- Servo test guide: https://book.servo.org/contributing/testing.html
- ServoDriver architecture: https://book.servo.org/design-documentation/servodriver.html
- Windows multiprocess PR: https://github.com/servo/servo/pull/37580
- Speedometer issue: https://github.com/servo/servo/issues/46070
- Crash reporting issue: https://github.com/servo/servo/issues/42928
- Verso archive: https://github.com/versotile-org/verso
- Servo penetration-test report: https://servo.org/files/ngie-servo-penetration-test-report-2024-1.0.pdf

</topic>
