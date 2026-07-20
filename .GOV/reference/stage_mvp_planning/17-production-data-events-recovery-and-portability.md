---
file_id: stage-production-data-events-recovery-and-portability
file_kind: reference-production-data-contract
updated_at: "2026-07-19"
status: research-hardened-proposed
wp_id: WP-1-Handshake-Stage-MVP-v1
---

<topic id="stage-production-persistence" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Stage production persistence

## Authority and storage rule

PostgreSQL records plus the canonical EventLedger are Stage durable authority. Browser user-data folders, engine navigation entries, renderer memory, UI rows, search projections, thumbnails, and diagnostic traces are materialized or derived state. ArtifactStore owns bytes and hashes. The later implementation must use the current product storage/event APIs after exact integration-baseline inspection; the names below are logical contracts, not pre-approved Rust module names.

## Logical table catalog

| Logical table | Core authority | Required keys and constraints |
|---|---|---|
| `stage_windows` | Durable window/workspace state. | `window_id`, workspace/project scope, active tab, revision, created/updated/closed timestamps; active tab must belong to window. |
| `stage_sessions` | Profile policy and trust class, not `ModelSession`. | `session_id`, profile class, engine preference, retention, encryption/key ref, trust class, revision; no raw secret values. |
| `stage_tabs` | Canonical tab identity and organization. | `tab_id`, window/session, URL/title facts, record state, folder/order, pin/mute, engine preference, revision; engine IDs excluded. |
| `stage_tab_history` | Versioned Stage-owned per-tab restore metadata that is portable/available from adapter. | `history_entry_id`, tab, sequence, URL/title, document/session-state reference, engine/provenance, created time; forward-prune semantics explicit. |
| `stage_visits` | Profile browsing history. | `visit_id`, session, URL/title, transition, visit time, source tab; excluded for private/ephemeral policy. |
| `stage_recently_closed` | Restorable tab/window snapshot references. | `closed_id`, kind, source IDs, snapshot/artifact ref, order, expiry, close reason. |
| `stage_folders` | Stage-local tab organization. | `folder_id`, window/workspace, parent, name, order, color, revision; acyclic tree and sibling-order constraints. |
| `stage_bookmarks` | Tab-independent bookmark authority. | `bookmark_id`, URL/title, folder/order, notes, created/source facts, revision; may link but cannot depend on live tab. |
| `stage_attachments` | Ephemeral-to-durable adapter mapping and health. | `attachment_id`, tab, adapter/version/build/digest, engine generation, profile materialization, process group, state, last health; only one active generation per tab. |
| `stage_control_leases` | Browser-control ownership. | target scope, actor/lane, fencing token, expiry, heartbeat, revision; monotonically increasing token per target. |
| `stage_action_intents` | Pre-dispatch durable command. | `intent_id`, actor, target, expected revision/generation, capability/policy, consequence, idempotency/correlation, deadline, source/sink facts, pre-observation hash. |
| `stage_action_receipts` | Dispatch/result/reconciliation. | intent FK, dispatch state/time, terminal class, postcondition, evidence refs, error, reconciliation revision; one authoritative terminal transition. |
| `stage_captures` | Stage capture intent, source truth, aggregate state, and lineage root; not an ArtifactStore manifest. | `capture_id`, source tab/session/navigation/generation, capture kind, requested/actual summary, completeness, policy/tool versions, state, idempotency/correlation, revision. |
| `stage_capture_parts` | Normalized result parts for one Stage capture. | `capture_part_id`, capture, role, shared opaque `ArtifactHandle`, observed hash/size/MIME snapshot, completeness, derivation parent, state; no bytes or raw path. |
| `stage_source_artifact_links` | Bounded source-to-result relationships across capture, download, ASR, translation, thumbnail, export, and intake. | source tab/selection, capture/workflow run, shared opaque `ArtifactHandle`, relation, source revision, idempotency, created/updated time; unique relation/idempotency contract. |
| `stage_bulk_runs` | Frozen canonical operation. | `bulk_run_id`, query/snapshot ref, operation, actor, total, cursor, concurrency, state, exact reconciliation. |
| `stage_bulk_targets` | Immutable target membership and per-item result. | bulk run + ordinal + target ID unique; pre-revision, result, post-revision, error/evidence. |
| `stage_jobs` | Correlation projection to Workflow Engine. | Stage operation, workflow/run/job IDs, source/target, state projection, last event sequence; Workflow Engine remains job authority. |
| `stage_profile_cleanup` | Deferred profile/data cleanup. | profile root reference, engine/process exit proof, attempts, next retry, terminal verification, error; no absolute machine path in portable exports. |
| `stage_outbox` | Atomic delivery when Stage state and EventLedger cannot share one transaction. | event ID/type/version, aggregate/revision, payload, created/published/attempt times; unique aggregate revision/type. |
| `stage_projector_checkpoints` | Deterministic projection rebuild. | projector/version, last event sequence, schema version, rebuilt time, error. |

Extension JSON is permitted only for non-authoritative adapter-specific facts and must carry an explicit schema/version. IDs, lifecycle, ownership, revisions, trust, retention, correlations, and security decisions cannot be hidden in unvalidated JSON.

## Index and query obligations

Indexes must be derived from measured queries, but the design must cover active windows/tabs, session/lifecycle, folder order, bookmark order, normalized URL, recently closed expiry/order, visit time, action status/actor/target/correlation, lease expiry, attachment generation/health, job correlation, capture source/state, capture-part role/handle, source-artifact relation, bulk-run state, cleanup retry, and unpublished outbox rows.

Every list API uses stable cursor ordering and a query snapshot/revision where consistency matters. Counts and bulk actions query canonical tables or frozen target rows, never rendered UI rows or a partial page.

## Transaction boundaries

- Create/mutate/close/restore operations update the aggregate revision and append the corresponding durable event in one transaction or via the transactional outbox.
- Compare-and-swap rejects stale revisions; no last-writer-wins for organization, lease, action, or capture authority.
- Lease acquisition increments a fencing token atomically. Expired actors cannot commit using an old token.
- Action intent becomes durable before adapter dispatch. Dispatch/terminal/reconciliation transitions are monotonic and idempotent.
- Bulk target membership is frozen before side effects; per-target state is independent so restart can resume exactly.
- Artifact creation uses streamed bounded staging then canonical ArtifactStore finalization; the Stage capture part points to a finalized shared handle or records partial/failure/reconcile-required state. Database success cannot claim missing bytes. Stage never stores payload bytes, scans the ArtifactStore, or invents another handle encoding.
- Workflow/job projection consumes ordered versioned events and may rebuild; it is not the workflow state authority.

## Event contract

Every Stage event has `event_id`, `event_type`, `schema_version`, `occurred_at`, `recorded_at`, aggregate ID/type/revision, actor/lane, causation, correlation, trace, workspace/project/session/window/tab as applicable, security/trust labels, and redacted payload.

Event families include record/window/session/tab/folder/bookmark/history transitions; attachment/lifecycle/navigation/focus/process health; capability/security/permission/request-policy decisions; control lease and operator takeover; action intent/dispatch/result/reconciliation; bulk plan/progress/result; capture/artifact/export; Downloader/ASR/project-intake correlation; runtime/update/migration/cleanup; diagnostic/health state. Event and error registries reject unknown unversioned variants.

Projectors are at-least-once consumers with event-ID deduplication, ordered aggregate revisions, explicit gap handling, poison-event quarantine, checkpointing, and deterministic rebuild/upcasters. Flight Recorder may sample diagnostics but EventLedger retains required durable product events.

</topic>

<topic id="stage-schema-migration-and-concurrency" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Schema migration, rollout, and concurrency

## Migration protocol

This protocol governs future version-to-version evolution of the new current Stage product after its canonical baseline exists. It does not grant compatibility to superseded pre-overhaul Stage implementations. WP-12 and other legacy Stage surfaces follow the removal/optional one-way-data-import plan in `15-product-topology-and-active-wp-migration.md`, with no default legacy read/write window.

1. Freeze integration baseline, current migration maximum, dependency versions, and current-data inventory.
2. Allocate unique migration numbers after active branches rebase; the later WP owns a migration conflict group and merge owner.
3. Expand with nullable/new structures and compatibility readers. Set lock/statement timeouts and avoid long `ACCESS EXCLUSIVE` operations.
4. Backfill in bounded idempotent chunks with cursor, progress, rate limit, cancellation, checksums/counts, and restart proof.
5. Create large indexes using the project-approved low-lock technique and validate query plans before switching traffic.
6. Enable canonical writes behind a versioned feature flag after forward/backward compatibility tests.
7. Keep legacy reads through one adapter for a time-bounded window; emit deprecation telemetry and reject new legacy writers.
8. Contract only after no-reader/no-writer proof, backup/restore rehearsal, rollback boundary review, and operator approval.
9. Record downgrade compatibility. If newer writes cannot be represented safely, older binaries refuse startup or enter read-only export/recovery mode.

## Concurrency rules

- Records expose an integer revision; APIs require `expected_revision` for mutation.
- Folder reordering and cross-window moves use one serializable operation or an equivalent tested ordering protocol; no duplicate/lost ordinal states.
- A tab has at most one active attachment generation. Late events from a prior generation are recorded as stale and cannot mutate current state.
- Window/tab close races with navigation, download, capture, lease, and editor embed-back through explicit transition tables and protected-work policy.
- Model/operator conflicts use fencing and operator priority, not focus or timing.
- Uncertain side effects remain blocked from blind replay.
- Canonical bulk runs are resumable and each target is idempotent by run/target/operation key.
- Projectors and search indexes may lag; UI/model responses expose source revision/checkpoint/staleness.

Property tests generate valid and invalid transition sequences, process/event reorderings, duplicate delivery, concurrent moves/close/restore, lease expiry, and crash/restart. Invariants are checked after every operation and after deterministic replay.

</topic>

<topic id="stage-backup-restore-and-portability" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Backup, restore, and portability

## Three separate products

1. PostgreSQL disaster recovery uses the project-approved encrypted base-backup plus WAL/PITR or managed equivalent. Restore drills include configuration/secrets not covered by database backup.
2. ArtifactStore backup protects bytes/manifests and records a consistency watermark with Stage/PostgreSQL/EventLedger state.
3. Operator portability uses a versioned Stage payload inside the existing Workspace Bundle/Export direction.

Raw live WebView2/Servo/CEF profile directories are not canonical backup. They may be optional encrypted migration inputs, but restore must succeed for canonical tabs/windows/folders/bookmarks/history/artifact links without them. Profiles are rematerialized; cookies or site storage transfer is explicit, partial, high sensitivity, and loss-reporting.

## Portable Stage payload

The payload includes format/schema versions, source product/engine versions, workspace/project identity, consistency watermark, windows/tabs/folders/bookmarks/order/history/recently-closed policy, Loom/artifact/job references, normalized URLs, settings allowed to travel, search-index rebuild instructions, content hashes, counts, capability-loss report, and encrypted optional secret/session section. Absolute paths and engine-native IDs are forbidden.

Import supports dry run, compatibility check, destination mapping, conflict policy, duplicate preview, bounded resumable execution, idempotency key, per-item results, rollback-before-commit, and post-import count/hash/link reconciliation. Unsupported newer schemas are rejected without partial mutation unless an explicit compatible subset is approved.

## Recovery objectives

Numerical RPO/RTO, backup frequency, retention, and restore time are operator-locked after measured baselines. Planning must not invent them. Required proof includes clean-machine restore, missing/corrupt segment, wrong key, partial ArtifactStore availability, PITR to selected watermark, database/artifact mismatch, profile rematerialization, 3,000-tab restore without navigation storm, and export of recoverable data from read-only safe mode.

</topic>

<topic id="stage-runtime-failure-and-health" status="research-hardened-proposed" version="v0.1" wp="WP-1-Handshake-Stage-MVP-v1" updated_at="2026-07-19">

# Runtime failure, recovery, and health

Failure domains are browser host/environment, renderer/content, GPU, network/utility, profile/storage, Stage host, database/EventLedger, Workflow/Downloader/ASR dependency, and external runtime/update. Correlated vendor notifications are deduplicated into one incident using engine generation, process mapping, time window, and causation.

Recovery state includes observed failure, affected targets, auto-recoverability, retry budget/backoff, crash-loop count, quarantine key, cleanup backlog, safe-mode recommendation, operator actions, evidence bundle, and terminal outcome. Quarantine may bind a URL/origin, profile, engine build, adapter capability, or migration rather than globally disabling Stage.

The health schema exposes adapter/runtime/profile/process mapping and hashes; capability/security posture; canonical/live/frozen/discarded counts; scheduler budgets/pressure; command/event queue depth and dropped/gap counts; attach/navigation/action/capture latency distributions; crash loops/orphans/cleanup backlog; profile/cache/artifact/database disk use; outbox/projector lag; uncertain-outcome backlog; permission/request-policy decisions; capture/downloader/ASR correlation; update/migration/backup status; and redaction/supply-chain state.

Safe mode restores canonical records and native UI without eager navigation, Stage Apps, model control, service-worker/background activation, or optional adapters. The operator can inspect, export, repair, clear/quarantine a profile, disable a feature/engine version, retry a migration, or collect a redacted support bundle.

</topic>
