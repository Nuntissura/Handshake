---
file_id: stage-release-packaging-update-and-support
file_kind: reference-release-operations-plan
updated_at: "2026-07-19"
status: research-hardened-proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-release-slices-and-promotion" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage release slices and promotion

## Non-deceptive closure model

The planning corpus distinguishes product release from umbrella-WP completion:

### `STAGE-WIN-BOOTSTRAP-PROD`

A Windows production release may use WebView2 when all required Windows-slice product features, primary operator journeys, authenticated-site disposition, security controls, failure recovery, packaging, updates, backup, diagnostics, accessibility, localization, manual, and support gates pass. Chromium-specific behavior remains below the adapter boundary and the UI visibly records engine/runtime provenance.

This milestone does not:

- make Chromium a coequal strategic backend;
- satisfy Servo gates;
- close the unified Stage WP;
- permit unsupported Google/YouTube authentication to be described as working;
- permit a production label based on a prototype navigation/capture subset.

### `STAGE-SERVO-RESTRICTED-ALPHA`

Servo can enter a restricted trusted/allowlisted-content alpha after exact dependency pins, embedding, process ownership, storage isolation/cleanup, request policy, accessibility, compatibility subset, crash/hang, packaging, supply chain, update/rollback, diagnostics, and soak evidence. The UI and capability manifest must enforce the restriction; arbitrary navigation cannot be enabled through a hidden setting or fallback.

### `STAGE-SERVO-ARBITRARY-WEB`

Windows arbitrary-web enablement is `SECURITY_BLOCKED` until an effective content-process sandbox exists and is independently validated with default-deny capability, filesystem/registry/process/window/IPC/network escape-oriented tests. Multiprocess topology or Rust memory safety cannot substitute for the gate.

### `STAGE-WP-COMPLETE`

The later unified WP closes only after current approved scope, complete legacy-source dispositions, full approved requirements, Servo strategic gates, current active-WP integrations, every official MT, spec propagation, manual, diagnostics, legacy Stage removal/optional data import, support, and task/WP/taskboard/traceability state synchronize from runtime evidence with no legacy Stage runtime authority.

</topic>

<topic id="stage-windows-packaging" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Windows packaging and platform qualification

Stage must use Handshake's project-wide delivery system. This planning corpus does not choose MSIX, sparse packaging, or an unpackaged installer in isolation. Before WP lock, inspect and record the actual installer/update/signing baseline and decide whether Stage's engine/runtime assets fit it.

The platform matrix covers:

- operator-approved Windows 10/11 builds, x64 and ARM64 if the product supports them;
- packaged/unpackaged identity, non-admin install, enterprise policy, proxy/TLS inspection, antivirus/application-control, and offline installation;
- clean install, first run, repair, upgrade, interrupted upgrade, side-by-side incompatibility, uninstall, reinstall, and user-data retention/removal choice;
- Unicode and long paths, non-default install/data roots, disk relocation, low disk, read-only/locked directories, and per-user/multi-user isolation;
- required WebView2 runtime detection and bootstrap, unavailable runtime, disabled updater, offline device, and minimum/feature-detected API behavior;
- signed Handshake binaries, adapters, helper executables, packages, manifests, and update metadata;
- Servo resources, certificates, fonts, locales, shaders, DLLs, crash symbols, and helper processes;
- optional Chrome-for-Testing/CEF assets kept out of the default package unless their separately approved lane requires them.

Every filesystem root is discovered from project/runtime configuration. Machine-local absolute paths never enter portable records or manifests.

</topic>

<topic id="stage-engine-runtime-updates" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Engine, runtime, and application updates

## WebView2

Evergreen WebView2 is Microsoft-serviced and normally supplies current security updates, but Stage cannot pin or roll back that shared runtime per installation. A running app continues using its existing environment until references are released or the app restarts. The production plan therefore includes:

- runtime presence/version/channel and API feature detection;
- preview-channel forward-compatibility qualification before stable rollout;
- `NewBrowserVersionAvailable` handling, state checkpoint, bounded operator-visible restart, and post-restart health proof;
- runtime-build kill/quarantine rules for a confirmed regression without disabling all Stage data access;
- a rollback distinction: Stage adapter/config/feature flag can roll back, state schemas use forward recovery, but vendor Evergreen runtime rollback is not an owned promise;
- Fixed Version only after an explicit escalation comparing more than 250 MB payload, app-owned security cadence, AppContainer/runtime permissions, redistribution, and update urgency.

## Servo and app-owned helpers

Servo, Chrome-for-Testing, and any later CEF assets use exact source/version/commit/toolchain/feature/lockfile/binary hashes; staged qualification; signed packages; retained previous known-good version where policy permits; state-format compatibility; feature kill switches; and rollback rehearsal. A new engine build cannot reuse old compatibility/security evidence after its freshness window or relevant source changes.

## Application/schema compatibility

Application rollout defines minimum and maximum readable schema/event versions, expand/backfill/cutover/contract compatibility, older-binary refusal, backup watermark, update cancellation boundaries, and recovery from crash between any two phases. New code must tolerate the prior supported data format during the compatibility window; destructive contract waits until the rollback boundary is accepted.

</topic>

<topic id="stage-supply-chain-and-licensing" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Supply chain, licensing, and provenance

Reuse the current OSS Component Register and product supply-chain surfaces. Every engine, crate, native library, helper executable, model, translation package, filter list, sanitizer/readability library, archive/parser tool, codec, font, locale resource, and runtime records:

- component ID, role, integration mode, source URL, version/commit/tag, toolchain/features, lockfile and binary/content hashes;
- SPDX license, notices, redistribution/codec/DRM posture, source-offer obligations if any, and package inclusion;
- update owner/cadence, vulnerability/advisory sources, supported branch, response/patch SLA to be operator-set, and end-of-life behavior;
- SBOM identity and signature/provenance/attestation;
- runtime network/telemetry/update behavior and privacy impact;
- qualification evidence and revalidation triggers.

Builds fail on missing required provenance, unknown binaries, unapproved license state, vulnerable disallowed versions, mismatched hashes, unsigned release artifacts, or SBOM/package drift. Numerical vulnerability response times remain operator decisions rather than invented acceptance targets.

</topic>

<topic id="stage-production-support" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Production support and incident operations

## Support surface

The shipped native UI exposes health summary, exact app/adapter/engine/runtime/schema versions, current capability/security posture, profile/process mapping, queued/uncertain actions, downloads/jobs, crash loops, cleanup backlog, update/migration/backup state, disk/resource use, and known quarantines. Operator actions include copy diagnostic ID, open manual topic, retry safe operation, pause models, revoke leases, restart environment/app, enter safe mode, export recoverable data, collect support bundle, and restore known-good configuration.

The support bundle is a versioned ArtifactStore artifact with manifest, hashes, build/runtime/profile/process topology, redacted logs/events/traces/receipts, health snapshots, relevant screenshots, reproduction manifest, crash metadata, migration/update state, and collection warnings. Raw cookies/tokens/passwords, hidden takeover input, unrestricted page bodies, client keys, or arbitrary local files are excluded. Page screenshots, crash dumps, network data, and captured payloads are separately consented high-sensitivity components with bounded retention.

## Incident lifecycle

1. Detect and correlate by stable incident ID.
2. Contain through actor pause, feature/adapter/runtime quarantine, session isolation, network/bridge block, or safe mode.
3. Preserve minimal redacted evidence and state watermark.
4. Recover by retry/recreate/rematerialize/rebuild/forward-fix/restore according to failure domain.
5. Verify canonical counts, revisions, artifact hashes, process cleanup, security posture, and primary journey.
6. Record root cause, affected versions, mitigation, revalidation triggers, and manual/support updates.

No incident workflow asks the operator to delete a whole profile/workspace as the first recovery step. Destructive reset requires previewed scope, export/backup option, typed consequence, confirmation, and verification.

## Operational drills

Pre-release and recurring drills cover browser/renderer/GPU crash, hang, runtime regression, profile lock/corruption, database failover/outage, EventLedger/outbox lag, disk full, bad filter/model update, certificate/proxy failure, download interruption, incomplete capture, orphan helper, prompt-injection/exfiltration incident, migration interruption, backup restore, and safe-mode export. Drill freshness is recorded and invalidated by relevant architecture/version changes.

</topic>
