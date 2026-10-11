---
file_id: BUILDER-BRIEF-HSK-STUDIO-KERNEL-BINDINGS
file_kind: builder_brief
updated_at: 2026-10-11T03:25:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-kernel-bindings
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-KERNEL_BINDINGS` / key `kernel_bindings` / crate `hsk-studio-kernel-bindings`; foundation MT `MT-35556` (read `implementation_steps` 1-4, `proof.checks`; depends_on MT-35554 session). Write scope `crates/hsk-studio-kernel-bindings/**` only.
- Responsibility (verbatim): "Host-neutral integration bridge port schemas and executable in-memory adapter consumer. Concrete AppState/SurrealDB/Argus/native adapters live later in Handshake product integration target and are excluded from this assets workspace."
- Foundation capability: "Execute one real bridge protocol proposal/authorization/cancel/diagnostic round trip through scoped transient adapter and fail closed on rejected grant; no claim of production storage/host wiring."
- Allowed deps (STUDIO_MODULES depends_on == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-session`, `hsk-studio-observe`. (MT build closure also lists chronicle/package; those arrive transitively through `session`, not as direct edges.)
- Hard prerequisite: `hsk-studio-session` must exist (it defines the injected `GrantPort`/`CommitPort`/`JobPort`/`Clock` traits - see `hsk-studio-session.md`). => wave 2, AFTER session.
- Forbidden: handshake_core, handshake_native, `AppState`, SurrealDB/RocksDB/SQLite, any Argus/AccessKit/egui/wgpu, tokio/sockets (the "bridge" is a byte-level protocol exercised in-process), storage claims (the transient adapter is memory only and must say so in `DESCRIPTOR`); edges outside `allowed`.
- Conventions (verified): edition 2024, rust-version 1.97.1, single-line inline `Cargo.toml` tables (`builders/WORKSPACE_RULES.md`), `[lib] path="src/lib.rs"`, `[[example]] name="kernel-bindings-consumer"`, `DESCRIPTOR`, `tests/foundation_contract.rs` + `bounded_case`; wire types follow folio/chronicle style (`serde(deny_unknown_fields)`, explicit `transport_version`, `schemars` NOT required).
- Prep refs: `docs_repo/studio-collaboration-2026-10-09/modular-authoring/shared-team/preparation.json` -> `shared_contract_proposals` (`actor`: portable attribution fields now, real authenticated principal later, never invented identities; `closure`; `proof_roles`); `.../embedding-team/preparation.json` `dependency_boundary` ("core binding consumes contract/document/proposal schemas only"), `host_shared_hook_assignment`, `release_precondition` (host adoption MTs MT-36200..36231 are LATER); `backend-redteam/pdf-audio-root-drill.json` `ownership_ports` (JobRuntime / ResourceBroker / SurrealDB-EventLedger / Argus-FR-Palmistry port semantics). Overlap rule: `session` owns only the traits and private test doubles; THIS crate owns the public wire envelopes and the executable transient adapter - do not duplicate.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-ARC-001 (l.185): `handshake_core::studio` is the integrated authority binding; it receives `AppState` SurrealDB client, sandbox, PromotionGate, CRDT by reference; asset crates own algorithms/contracts through typed ports; immutable inputs and PROPOSED outputs carry canonical schemas, IDs, units and resource context; only the kernel binding may persist accepted authority; preserve `StudioEngineError` identifier and `event_family` constants at the kernel-binding contract owner (do not rename). STU-ARC-002/003/004: no GPU deps in core; event variants/families; prefixed domain IDs preserved at every API and receipt boundary; every authority record needs an `event_ledger_event_id` (host side - the bridge only carries it opaquely in receipts).
- STU-ARC-005 (l.193) + STU-PW-001..004: model-authored mutation = sandbox -> `StudioValidationDescriptor` catalog -> `PromotionGate` (`PromotionDecisionV1: Accepted`) -> authority; no self-approval; rejected proposals persist as replayable evidence. STU-ARC-011/020: expected-revision/footprint preconditions, idempotency bound to principal+resource/action+payload hash+revision, post-commit cancel cannot erase, outcome-unknown reconciles via lookup, lease epochs fence stale completions. STU-SDB-002/004/005 (l.36/60/71): SurrealDB-only authority, EventLedger atomic transaction + idempotency claim, `ResourceBroker` authorization gate - all HOST concerns that the bridge only describes, never implements.
- STU-ARC-015 (visual receipts), STU-ARC-014 (Controls/Argus descriptors) feed the `diagnostic` envelope; Observe is the diagnostic sink adapter (`hsk_studio_observe`).
- Later: STU-ARC-016/017 foundations & MT preservation, host adoption (MT-36200+), concrete adapters (AppState/SurrealDB/Argus) - explicitly OUT of this workspace.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust; ~800-1000 LOC; `src/{lib,wire,host,adapter}.rs`)
Reuse (verified): `hsk_studio_accord::{ActorContext (new/getters), DomainId, CancellationToken, ValidationError}`; `hsk_studio_folio::{Snapshot, Budget, Unresolved/Resolver}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`; from `hsk-studio-session` (read its brief/lib once landed): the `GrantPort`, `CommitPort`, `CommitOutcome`, `IdempotencyKey`, `Clock` traits/types.
1. Versioned wire envelopes (host-neutral; names are code names, canonical Studio schema ids are NOT invented): `pub const BRIDGE_VERSION: u32 = 1;` `Envelope{bridge_version:u32, correlation_id:u64, idempotency_key:String, actor:ActorWire, resource_id:String, expected_revision:u64, lease_epoch:u64, body:Body}` with `enum Body{ Grant(GrantMsg), Proposal(ProposalMsg), Promotion(PromotionMsg), Resource(ResourceMsg), Diagnostic(DiagnosticMsg) }` and responses `Reply{correlation_id, result:ReplyBody}` (`GrantMsg::{Request{action}, Decision{granted:bool, reason}}`, `ProposalMsg{proposal_id (SEPR DomainId), payload_sha256, base_revision, footprint}`, `PromotionMsg::Decision{Accepted{receipt}|Rejected{code}|Stale{current_revision}|StaleEpoch|Unknown}`, `ResourceMsg{lease epoch/bytes}`, `DiagnosticMsg{outcome, code}` mapped to observe outcomes). `encode(&Envelope)->Result<Vec<u8>>` / `decode(&[u8], &Limits)->Result<Envelope>` via the pinned `serde_json`: reject unknown fields, wrong `bridge_version`, duplicate keys (reuse folio's `UniqueValue` approach or a pre-scan), trailing bytes, > `Limits.max_bytes`, invalid `DomainId` (use `DomainId::parse`).
2. Host-neutral port trait + executable transient host: `pub trait HostBridge{ fn handle(&self, request:&[u8]) -> Vec<u8>; }` (byte-level, so a later AppState adapter is a drop-in); `pub struct TransientHost` (memory only; says so in `DESCRIPTOR`) that really enforces: per (actor principal, resource, action) grants with `revoke()`, expected-revision compare, lease-epoch fence, idempotency table bound to principal+resource+action+payload hash (+revision), monotonically increasing `committed_revision` per resource, receipt ids, and a `FaultPlan` for tests (`DropAck`, `RevokeBeforeCommit`, `BumpEpoch`). Absent/revoked grant => `Decision{granted:false}` AND no state change (fail closed).
3. Session adapter: `pub struct BridgePorts<H: HostBridge>` implementing session's `GrantPort` and `CommitPort` by encoding envelopes, calling `HostBridge::handle`, decoding replies and mapping `PromotionMsg::Decision` -> `CommitOutcome::{Committed, Rejected, Stale, StaleEpoch, Unknown}` (a malformed/missing reply = `Unknown`, NEVER `Rejected`); `lookup(key)` sends a read-only reconcile request. Exercise end to end through `session::Session::submit` (proposal -> authorization -> decision -> receipt) with the real chronicle provider.
4. Cancellation + diagnostics: cancel before send => nothing encoded; cancel after `Accepted` => committed receipt returned (post-commit cancel cannot erase); every terminal result is also reported as a `Diagnostic` envelope and delivered through `Observe` (Success/Canceled/Failure(Validation|Unavailable)) with no private text. `pub fn diagnostic_outcome(&ReplyBody) -> observe::Outcome`.
5. `DESCRIPTOR` JSON including a `later_host_adapters` array describing EXACTLY the not-implemented canonical consumers and their obligations (AppState shared client by reference, ResourceBroker grant check, sandbox + validation catalog + PromotionGate decision, EventLedger atomic append with idempotency claim and `event_ledger_event_id` in receipt, CRDT draft boundary, Argus/Flight Recorder/internal_diagnostics/Palmistry diagnostic sinks) each tagged `NOT_IMPLEMENTED_HOST_ONLY`, plus example `kernel-bindings-consumer` (`--descriptor`, `--roundtrip`, `--revoke`, `--drop-ack`, `--cancel`) printing the receipt chain.
Oracle: independent read of `TransientHost` state after each scenario (committed revision, receipt count, idempotency table size) rather than the adapter's own return value; byte-level golden envelope strings checked in the test file.

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- `serde 1.0.229` (`std`,`derive`, no default features), `serde_json 1.0.151` (`std`): both already steward-pinned and pure-rust (registry metadata of folio/chronicle; `kind = "pure-rust"`); request via dep-request and probe `approved` extension for `hsk-studio-kernel-bindings`. `sha2 0.10.9` (`force-soft`, pure-rust, workspace-locked) for `payload_sha256` only if session does not export its canonical payload-hash helper (prefer reusing session's).
- Rejected: `schemars` (not needed), tokio/hyper/tonic/axum (no real sockets), `uuid` (IDs minted by the host; this crate validates `DomainId`s only), anything that names `AppState`/SurrealDB.
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-kernel-bindings --tests` (then clippy, focused test).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case`, no network, no files)
1. `roundtrip_proposal_authorization_decision_receipt`: through `Session::submit` + `BridgePorts<TransientHost>`: revision N -> N+1, receipt id present, `TransientHost` independently reports committed revision N+1 and exactly one receipt; golden envelope JSON for the request matches byte-for-byte.
2. `fail_closed_on_absent_or_revoked_grant`: no grant and revoked-mid-flow both return denied/rejected, host revision and receipt count unchanged, retry after revocation re-checks grant (does not replay the receipt).
3. `stale_epoch_duplicate_key_and_unknown_outcome`: stale `expected_revision` => `Stale`; bumped lease epoch => `StaleEpoch`; same key + changed payload => conflict; `FaultPlan::DropAck` => `Unknown` (distinct from `Rejected`), `reconcile` finds the receipt, applied exactly once.
4. `cancel_semantics_and_diagnostics`: cancel before send => zero bytes sent; cancel after `Accepted` => committed receipt returned; each terminal path emits exactly one `Diagnostic` envelope and one observe frame with the right `Outcome`.
5. `wire_strictness`: unknown field, wrong `bridge_version`, duplicate key, trailing bytes, oversize, invalid `DomainId` prefix all rejected with distinct codes; `DESCRIPTOR` parses as JSON and lists every `later_host_adapters` entry as `NOT_IMPLEMENTED_HOST_ONLY`.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No AppState/SurrealDB/EventLedger/ResourceBroker/PromotionGate/CRDT implementation, no Argus/AccessKit/UserManual wiring, no persistence of any kind, no real transport, no new canonical schema ids, no host adoption (MT-36200..36231 are the separate embedding-team step), no GUI.
- Wave: 2, strictly after `session`. Report READY_FOR_VALIDATION only; never self-PASS; never claim production storage/host wiring.

</topic>
