---
file_id: BUILDER-BRIEF-HSK-STUDIO-SESSION
file_kind: builder_brief
updated_at: 2026-10-11T02:50:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-session
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-SESSION` / key `session` / crate `hsk-studio-session`; foundation MT `MT-35554` (read `execution.implementation_steps` 1-4, `proof.checks`). Write scope `crates/hsk-studio-session/**` only.
- Responsibility (verbatim): "Headless single-document operation orchestration, provider registration and test ports. Multi-output delivery captures immutable revision/presets/output verification and uses existing Job Runtime via a port for queue/watchfolder/cancel/retry, without a second job database or queue authority. No built-in all-engine mega dependency."
- Foundation capability: "Actual document-scoped admission/proposal/cancel/retry via injected canonical job/resource/commit ports with bounded work and stale handling."
- Allowed deps (STUDIO_MODULES depends_on == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-chronicle`, `hsk-studio-package`, `hsk-studio-observe`. NOTE `package` does not exist yet (brief `hsk-studio-package.md`, wave 1 for package) => session is wave 2; capabilities 1-4 below do NOT need package, so the builder may start with accord+folio+chronicle+observe and add the `package` path dep when it lands.
- Forbidden: handshake_core/native/SurrealDB/RocksDB (ENG-DAG-FORBID-001), second scheduler/queue/job DB (shared_owners.jobs: existing Job Runtime via typed port only), duplicated EventLedger/CRDT/history authority (ENG-DAG-FORBID-003), engine crates (render/raster/vector/...: "no all-engine mega dependency"), threads/async runtimes in slice 1 (synchronous, `Send + Sync` ports), edges outside `allowed`.
- Conventions (verified): edition 2024, rust-version 1.97.1, single-line inline `Cargo.toml` tables (`builders/WORKSPACE_RULES.md`), `[lib] path="src/lib.rs"`, `[[example]] name="session-consumer"`, `DESCRIPTOR`, `tests/foundation_contract.rs` + `bounded_case`. Existing-crate patterns to copy, not reinvent (`research/existing-api-digest.md` "Cross-crate patterns"): RAII reservation that retires on Drop, caller-implemented `&dyn` ports, two-phase prepare -> serialized finalize -> deliver with a pending token + `reconcile`, stable `code()` strings.
- Prep refs: `.GOV/docs_repo/studio-collaboration-2026-10-09/backend-redteam/pdf-audio-root-drill.json` -> `ownership_ports` (JobRuntime admit/cancel/status; ResourceBroker grants; SurrealDB/EventLedger commit with expected revision + idempotency; Argus/Flight Recorder), `foundation_execution_floor`; `backend-redteam/backpressure-architecture-review.json` (bytes AND slots bounded, retired-worker permits held until work ends; preview work may be superseded with an explicit stale outcome, committed edits never dropped); `repass-2026-10-09/filmcraft-root-drill.json` FILM-ROOT-01 (staged edit + one document owner; a Session per agent would FORK authority) and FILM-ROOT-11 (idempotency, bounded queues, Backpressure/RetryAfter/Busy receipts; replay of mutations after unknown completion is forbidden without lookup).

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-ARC-020 (l.249): idempotency binds authenticated principal + resource/action + canonical payload hash + target revision/read-write preconditions; identical authorized retry returns the canonical receipt; same key with changed payload/context CONFLICTS; reauthorization mandatory even for known receipts; post-commit cancellation cannot erase an accepted edit; outcome-unknown commit/ack failures reconcile canonical authority (rediscover receipt, never apply twice); lease epochs fence stale worker completions; shutdown closes admission and drains/checkpoints accepted outcomes within a declared deadline via reserved bounded paths.
- STU-ARC-011 (l.217): document-scoped actor serializes accepted commits; compute consumes immutable snapshots and PROPOSES patches; read/write footprints + expected revision vectors (not blanket whole-document CAS); commit rechecks current grants and preconditions. STU-ARC-008/012 (l.207/219): bounded admission by actor/document/resource, job count, bytes in flight; overload outcomes, fairness, cancellation/deadlines, stale-revision rejection, release on EVERY exit; waiting for a budget must NOT hold authority/lease locks; partial multi-resource acquisition releases before waiting; reserved bounded cancel/completion paths.
- STU-ARC-005 (l.193) / STU-PW-001..004: model mutations are `StudioEditProposal` (`hsk.studio.edit_proposal@1`, accord prefix `SEPR`) -> sandbox -> validation -> PromotionGate; replayable from receipts; no self-approval. STU-PAR-001..005: one document = one job/lease scope; multi-model on one file mediated, no silent overwrite; process ownership. STU-QUIET-001..004: headless, no focus/OS-input.
- Later: STU-HIS-* history, STU-DS-163 conflict semantics, multi-output delivery/watchfolder (STU-VID-064 export queue headless/observable), STU-MDL/STU-UNI/STU-MAN. MT corpus: session feature MTs are in the STU-PAR/PW/HIS block (MT-33640..33657, from the title scan; verify before use).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only, synchronous, ~900-1200 LOC; `src/{lib,ports,admission,idempotency,session}.rs`)
Reuse (verified): `hsk_studio_folio::{Snapshot (document().revision, encoded_bytes()), Budget, Resolver, Unresolved}`; `hsk_studio_chronicle::{prepare, Patch, RevisionEntry, Prepared, decode_patch, Budget as ChronicleBudget, Diagnostic}` (real proposal provider: name/visible property patches, see `crates/hsk-studio-chronicle/examples/chronicle-consumer.rs`); `hsk_studio_accord::{ActorContext, CancellationToken, DomainId (SDOC/SEPR/SHIS prefixes), ValidationError}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`.
1. Injected ports (traits only in this crate; no storage): `trait GrantPort{ fn check(&self, actor:&ActorContext, resource:&DomainId, action:Action) -> Result<GrantToken, GrantError> }` (fresh check on EVERY submit and retry; `GrantError::{Revoked,Missing}`), `trait CommitPort{ fn commit(&self, c:&CommitRequest) -> CommitOutcome; fn lookup(&self, key:&IdempotencyKey) -> Option<CommitReceipt> }` with `enum CommitOutcome{Committed(CommitReceipt), Rejected(Code), Stale{current_revision:u64}, StaleEpoch, Unknown}`, `trait JobPort{ fn admit(&self, scope:&JobScope, cost:Cost) -> Result<JobTicket, Overload>; fn release(&self, t:JobTicket); }` (the existing Job Runtime is OUTSIDE; session never owns a queue), `trait Clock{ fn now_ms(&self)->u64 }` (test clock), `trait ProposalProvider{ fn propose(&self, snap:&Snapshot, params:&[u8], cancel:&CancellationToken) -> Result<Proposal, ProviderError> }` registered by id in `ProviderRegistry` (instance-owned, enumerable, unknown id -> typed `ProviderUnavailable`). Ship `ChronicleProvider` (calls `chronicle::prepare` on the immutable snapshot) as the real provider; keep in-memory commit/grant/job doubles PRIVATE to `tests/` and the example (they must really enforce revision, epoch and idempotency - not canned). The PUBLIC executable transient adapter that implements these ports belongs to `hsk-studio-kernel-bindings` (MT-35556), not here - do not duplicate it.
2. Bounded admission, no waiting: `Admission::new(capacity:u32, byte_budget:u64)`; `try_acquire(cost) -> Result<Permit, SessionError::Overloaded{retry_after_ms}>` immediately (never blocks); `Permit` RAII releases on every exit incl. panic-free error paths; capacity 1 => second concurrent submit gets `Overloaded`; permits are NOT held while calling `CommitPort` waits (acquire after grant, release before reconcile waits).
3. Document-scoped submit/cancel/retry: `Session::new(doc:Snapshot, actor:ActorContext, epoch:u64, ports:Ports, admission, clock)`; `submit(&mut self, req:SubmitRequest, cancel:&CancellationToken) -> Result<SessionReceipt, SessionError>` with `SubmitRequest{key:IdempotencyKey, action:Action, provider:&str, params:Vec<u8>, expected_revision:u64}`. Order: grant check -> idempotency lookup (key bound to principal+resource+action+SHA-256(canonical payload)+expected_revision: same key + same bound tuple + authorized -> return stored canonical receipt; same key + different payload/context -> `IdempotencyConflict`) -> admission -> provider.propose on the IMMUTABLE snapshot (poll cancel) -> revision/epoch precheck (`StaleRevision`, `StaleEpoch` for a worker completing under an old epoch) -> `CommitPort::commit` -> map outcome: `Committed` => advance local snapshot to the committed document and store receipt; `Unknown` => `SessionError::OutcomeUnknown` + `pending` entry; `reconcile(&mut self)` calls `CommitPort::lookup(key)` (found => adopt receipt exactly once; absent => safe retry) - never re-applies blindly. Cancel before commit => nothing published, permit released; cancel AFTER `Committed` returns the committed receipt (post-commit cancel cannot erase the edit, distinct result `CancelTooLate`).
4. Shutdown drain: `close_admission()` (further submits => `Closed`), `drain(&mut self, deadline_ms:u64) -> DrainReport{completed, reconciled, abandoned}` using the injected `Clock`; asserts cancel acknowledgement <= 1000 ms and drain <= 2000 ms on the TEST clock (proposed bounds from MT-35554), unresolved `Unknown` outcomes are reported, not dropped.
5. (After `package` lands, wave 2) `Session::export_checkpoint(&self, assets:&dyn package::AssetSource, limits:&package::Limits, cancel) -> Result<Vec<u8>, SessionError>` writing the committed snapshot through `package::write_package` and re-reading it with `read_package` for output verification (immutable revision + verification in the receipt). Plus `DESCRIPTOR`, `report(...)` into `Observe` (Success/Canceled/Failure(Validation|Unavailable|Loss)) and example `session-consumer` (`--descriptor`, `--rename SDOC.json NAME --capacity 1 [--cancel] [--retry] [--revoke-grant]`).
Fixture oracle (MT-35554): capacity-1 admission, one proposal completes, one canceled, one authorized retry returns the SAME receipt, a revision transition (rev N -> N+1) read back independently from the final `Snapshot`/receipt; negatives each return a DISTINCT error code: capacity-1 overload, grant revoked between retry attempts, same key changed payload, stale worker completion (old epoch), lost commit acknowledgement (`Unknown` then `reconcile` finds receipt, applied once), post-commit cancel.

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- Slice 1 needs ONE crate: `sha2 0.10.9` (`default-features=false`, `features=["force-soft"]`, pure-rust; already locked in the workspace for nib/type; steward must add `hsk-studio-session` to the probe `approved` list) for the canonical payload hash; or `sha2 0.11.0` as prism uses - pick the workspace version steward prefers. File a dep-request line first; do NOT hand-roll a non-cryptographic hash for idempotency binding (needs collision resistance) - develop the other capabilities while the steward merges the pin.
- serde/serde_json (`1.0.229`/`1.0.151`, pure-rust, already registry-pinned for folio/chronicle) only if receipts need JSON; otherwise hand-format. Rejected: tokio/async-std/crossbeam (no async runtime; synchronous ports), `uuid` (IDs come from accord `DomainId::parse`; this crate never mints identifiers - it is told idempotency keys and proposal ids by the caller).
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-session --tests` (then clippy, focused test).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case`, test clock only - no sleeps)
1. `submit_commits_and_retry_returns_same_receipt`: chronicle rename proposal commits rev N -> N+1; independent read of final snapshot shows new name; identical retry returns the stored receipt (commit port called exactly once) AFTER a fresh grant check (grant call count == 2).
2. `overload_and_release`: capacity 1 with a held permit => second submit `Overloaded` immediately; after the first finishes (success AND error AND cancel paths) permits are released (`in_flight == 0`).
3. `idempotency_and_grant_negatives`: same key + changed payload => `IdempotencyConflict`; revoked grant on retry => `GrantRevoked` even though a receipt exists; stale expected revision => `StaleRevision`; old-epoch completion => `StaleEpoch`; all codes distinct.
4. `unknown_outcome_reconcile_and_post_commit_cancel`: `CommitOutcome::Unknown` => `OutcomeUnknown`; `reconcile` adopts the receipt found by `lookup` and the document advanced exactly once; cancel after commit => `CancelTooLate` with the committed receipt; cancel before => nothing published.
5. `shutdown_drain_deadlines`: `close_admission()` then submit => `Closed`; `drain` on the test clock finishes <= 2000 ms and reports unresolved `Unknown` entries; cancel acknowledgement <= 1000 ms.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No persistence, no EventLedger/CRDT/SurrealDB, no real Job Runtime/scheduler/queue/watchfolder (ports only), no multi-output delivery pipeline or presets (later; STU-VID-064 / STU-IO export), no multi-document or multi-model arbitration (STU-PAR-002/003 are host-scoped), no sandbox/PromotionGate (host), no GUI/Controls, no threads/async, no minting of identifiers.
- Wave: 2 (needs `package` for capability 5; capabilities 1-4 compile against existing crates). Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
