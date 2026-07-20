---
file_id: stage-rw-019-020-production-data-and-release-operations
file_kind: research-note
updated_at: "2026-07-19"
status: primary-and-local-source-basis-complete-validation-pending
wp_id: WP-1-Handshake-Stage-MVP-v1
verification_status: primary-and-local-source-basis-complete-validation-pending
---

<topic id="stage-rw-019-production-data-and-recovery" status="primary-and-local-source-basis-complete-validation-pending" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# RW-019 production data, migration, and recovery

## Sources checked

- `STAGE-SRC-LOCAL-020` through `STAGE-SRC-LOCAL-027`: current product and active-WP storage/event/migration topology.
- `STAGE-SRC-WEB-066`: Chromium session-history persistence model.
- `STAGE-SRC-WEB-077`: WebView2 process failures.
- `STAGE-SRC-WEB-080`: PostgreSQL table-alter locking.
- `STAGE-SRC-WEB-081`: service-worker background lifecycle.
- `STAGE-SRC-WEB-082`: WebView2 user-data folders.
- `STAGE-SRC-WEB-084`: PostgreSQL point-in-time recovery.

## Patterns selected

- PostgreSQL records plus EventLedger are authority; browser profiles/UI/search/diagnostics are materialized or derived state.
- State mutation and event append share a transaction or transactional outbox; projectors deduplicate, checkpoint, upcast, and rebuild.
- Optimistic revisions, monotonic fencing tokens, frozen bulk targets, durable pre-dispatch intents, and uncertain-outcome reconciliation prevent concurrency/retry corruption.
- Future upgrades within the new current Stage product use expand, bounded resumable backfill, cutover, and delayed contract. Superseded pre-overhaul Stage surfaces use removal/replacement plus optional one-way import of designated real operator data, not a compatibility window.
- Database, ArtifactStore, and operator portability are separate backup/restore concerns with a consistency watermark.
- Profile directories are rematerialized runtime state, not canonical backup authority.

## Rejected options

- Treating engine history/profile files as the Stage database.
- Updating PostgreSQL state and emitting durable events in unrelated best-effort operations.
- Blind retry after an action timeout.
- Renumbering WP-12 migrations or preserving compatibility aliases instead of inventorying and removing/replacing the full legacy cross-table graph; any designated real operator data still requires verified one-way import.
- Backing up a live user-data folder as the only recovery path.

## Validation plan

Property/replay tests, legacy-surface absence scans, optional designated-real-data import from a prior snapshot, crash/interruption fault injection, clean-machine database/artifact/portable-payload restore, profile rematerialization, 3,000-tab no-storm restore, and count/hash/link reconciliation.

</topic>

<topic id="stage-rw-020-packaging-update-and-support" status="primary-and-local-source-basis-complete-validation-pending" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# RW-020 packaging, updates, supply chain, and support

## Sources checked

- `STAGE-SRC-WEB-038` and `STAGE-SRC-WEB-076`: WebView2 distribution and runtime update modes.
- `STAGE-SRC-WEB-077`: process/restart behavior.
- `STAGE-SRC-WEB-083`: Windows accessibility validation.
- `STAGE-SRC-WEB-085`: supply-chain scan/SBOM artifact support.
- current product diagnostics, UserManual, and supply-chain source topology under `STAGE-SRC-LOCAL-021`.

## Patterns selected

- Use Handshake's project-wide installer/update/signing system after the exact baseline is approved; Stage does not invent a separate delivery stack.
- Evergreen WebView2 receives external vendor updates. Stage qualifies preview/stable versions, feature-detects APIs, checkpoints state, restarts to adopt updates, and can quarantine adapter/config features; it does not promise per-app vendor-runtime rollback.
- App-owned Servo/CfT/CEF assets use exact pins/hashes, signed staged rollout, previous-known-good retention where allowed, state compatibility, and rollback drills.
- Every component/list/model/codec/runtime enters the existing component register, notices, SBOM, advisory policy, signing, and revalidation flow.
- Safe mode, structured health, redacted support bundle, and incident/drill lifecycle are shipped product surfaces.

## Rejected options

- Calling an Evergreen vendor update reversible by Stage.
- Selecting Fixed Version only for determinism without accepting payload/security-update ownership.
- Hiding packaging, migration, backup, localization, or support inside final integration cleanup.
- Treating logs or crash dumps as automatically safe to export.

## Validation plan

Clean-machine/non-admin/offline/enterprise-policy install matrices; signed/SBOM/package drift proof; runtime update/restart/regression quarantine; safe-mode and support-bundle secret scan; recurring failure, incident, and restore drills.

</topic>
