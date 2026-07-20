---
file_id: stage-validation-evidence-and-production-gates
file_kind: reference-validation-and-evidence-contract
updated_at: "2026-07-19"
status: research-hardened-proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-validation-harness" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage validation harness

## Evidence authority

Planning documents do not pass production gates. A future gate passes only from a target-branch/build run whose command, environment, inputs, versions, fixture hashes, results, logs, screenshots, traces, and artifacts are recorded in a versioned evidence manifest and accepted by the applicable existing validator/operator surface.

Exact Stage commands cannot truthfully be named before implementation entrypoints exist. The future WP/MT generator must bind every `command_id` in `production-gates.yaml` to an exact non-interactive repository command and reject `TBD`, wildcard-only, zero-test, skipped-only, or uncollected-evidence commands.

## Harness contract

Every command supports:

- deterministic fixture/manifest ID and content hash;
- exact app/adapter/engine/runtime/schema/build commit and binary hashes;
- test target OS/architecture/hardware/power/display/locale/timezone/network/profile class;
- deterministic clock/IDs/random seed where applicable;
- structured JSON result plus human-readable projection;
- per-case result, duration, retry count, expected failure, quarantine, and reason;
- screenshots, video when needed, state snapshots, logs, traces, process/profile mapping, network policy decisions, receipts, and artifact handles;
- no-test/zero-match/skipped-only guard;
- timeout and cleanup verification;
- secret/redaction scan;
- evidence signature/hash and freshness/expiry.

Test fixtures never reuse the operator's real default browser profile, credentials, projects, or uncontrolled local paths.

## Test layers

1. Pure schemas/state machines: serialization, compatibility/upcast, transition/property, URL/policy/parser, scheduler, reconciliation, redaction.
2. Fake adapter conformance: deterministic commands/events, capability states, stale generation, cancellation, timeouts, reorder/drop/duplicate, process failure, no silent fallback.
3. Controlled real-engine fixtures: local TLS/proxy/auth/private-network/service-worker/media/download/permission/popup/frame/Stage-App/capture sites.
4. Product integration: PostgreSQL/EventLedger/outbox/projectors, `StageCaptureCoordinator` against the shared canonical ArtifactHandle plus streamed ArtifactStore ingest/finalize/abort/reconcile/materialize contracts, Workflow/Downloader/ASR, WP-1 model-control fake/frozen baseline, legacy WP-12 Stage removal plus optional real-data import, newly specified editor integration if selected, and Loom/Atelier/Lens/CKC consumer fixtures.
5. Native visual/accessibility: Argus/visual-debug route, UIA/assistive technology, keyboard, focus, DPI, multi-monitor, text scale, high contrast, RTL/long strings, no focus theft.
6. Fault/security: fuzzing, malformed content, injection/exfiltration, SSRF/DNS rebinding, bridge confusion, cert/proxy, disk/memory/process/database/event/update/migration/backup failures.
7. Scale/performance/soak: 1/10/100/1,000/3,000/10,000-record curves, mandatory realistic 3,000-plus single- and multi-window fixtures, machine-wide bounded live/background sets, zero-per-record dormant work, pin-versus-keep-live, incremental projections, restore storms, scheduler fairness, service-worker/process attribution, no ArtifactStore hot-path scan, and handle/process/task/timer/subscription/UI-allocation/memory/disk/network trends.
8. Packaging/operations: clean machine install/update/uninstall, missing/offline runtime, signatures/SBOM, safe mode, support bundle, restore drill.
9. Live-site monitoring: expiry-bound representative journeys; never the sole deterministic release authority.

</topic>

<topic id="stage-controlled-fixture-corpus" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Controlled fixture corpus

The fixture service is owned, versioned, reproducible, network-isolated where appropriate, and can issue test roots/certificates. It includes:

- navigation, redirects, same-document and frame history, dynamic DOM/shadow DOM, popups, JavaScript dialogs, `beforeunload`, error pages, offline/reconnect;
- form, password, passkey/WebAuthn test mode, MFA test mode, HTTP auth, client cert, upload/file chooser, drag/drop, clipboard;
- proxy/PAC/auth, TLS valid/expired/self-signed/wrong-host/revoked fixtures, HSTS/mixed-content/insecure form, IPv4/IPv6/loopback/link-local/private DNS-rebinding targets;
- permissions for camera/mic/geolocation/notifications/clipboard/MIDI/local fonts/screen capture/window management/automatic downloads and top-frame/subframe variations;
- service workers, cache/IndexedDB/storage, WebSocket/WebTransport, WebRTC, screen share, push/background capability gaps;
- downloads with range/no-range, disconnect, timeout, auth, cross-origin redirect, wrong length/hash, malicious/safe test file, blocked policy, disk full, collision, long/invalid/path-traversal names;
- HTML/readability/Markdown/PDF/media/codec/DRM/3D/WARC/capture, including malformed, oversized, deep nesting, decompression bomb, parser crash/hang, active-content injection, inaccessible content, and partial resources;
- browser-agent injections in DOM, accessibility, CSS-hidden text, image/OCR, PDF, iframe, download, service worker, search result, capture, bookmark, Loom-linked content, and cross-tab context;
- Stage App signed/invalid/expired/revoked/downgrade packages, origin/navigation confusion, oversized/malformed bridge messages, dependency migration, outbound denial;
- 3,000-tab import with hierarchy/order/duplicates/history/bookmarks, cold restore, pin-versus-keep-live, detached downloads, bulk operations on offscreen rows, multi-window redistribution under one global ceiling, service-worker/background-process attribution, no per-record host work or ArtifactStore scan, crash mid-import, and a 10,000-record stretch for hidden O(n) detection.

CAPTCHA uses provider test keys or an owned deterministic challenge fixture; the product behavior is operator takeover, never bypass.

</topic>

<topic id="stage-evidence-manifest" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Evidence manifest and gate rules

```yaml
schema_id: handshake.stage-evidence-manifest.v1
evidence_id: stable-id
gate_id: STAGE-GATE-...
command_id: STAGE-CMD-...
started_at: timestamp
completed_at: timestamp
result: PASS|FAIL|BLOCKED|INVALID
source_commit: git-sha
dirty_state: clean|declared
build_artifact_hashes: []
app_version: string
adapter_versions: []
engine_runtime_versions: []
database_schema_version: string
event_schema_versions: []
fixture_manifest_id: string
fixture_hash: sha256
environment_manifest: artifact-handle
case_counts:
  selected: 0
  executed: 0
  passed: 0
  failed: 0
  skipped: 0
  expected_failed: 0
evidence_artifacts: []
redaction_scan: PASS|FAIL
cleanup_verification: PASS|FAIL
known_exceptions: []
expires_at: timestamp
revalidation_triggers: []
signed_manifest_hash: sha256
```

A gate is invalid when selected or executed count is zero, required artifacts are missing, results are only skipped/expected failures, cleanup or secret scan fails, versions/fixture hashes are absent, the build does not match the candidate, evidence expired, a relevant dependency changed, or any required subgate is failed/blocked.

Performance targets are operator-locked after baseline distributions on named hardware. The harness records median/tails/outliers and resource time series rather than selecting one favorable run. Comparative claims name identical fixture, versions, hardware, power mode, and run protocol.

</topic>
