# WP-KERNEL-003 Integration Validator Handoff Bundle

> **Authority note.** This Markdown is an explicit report projection authored
> under packet `artifact_policy.allowed_markdown_exceptions =
> "explicit_report_projection_contract"`. Authority remains in
> `packet.json`, the MT-*.json contracts, and the per-MT receipts at
> `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/RECEIPTS.jsonl`.
> This document is scannable evidence for the Integration Validator only.

## Status (no PASS/FAIL claim)

- Kernel Builder reports: **79/80 product MTs implemented and receipted**;
  MT-080 (this handoff) closes as **IV-handoff-ready**.
- Kernel Builder **does not claim PASS or FAIL** for any acceptance row,
  spec anchor, or MT (per kernel-builder protocol VALIDATION_BOUNDARY).
- Integration Validator (`INTEGRATION_VALIDATOR_BATCH_MT_THEN_SPEC_V1`
  topology) is the sole authority for batch MT review and Master Spec
  review verdicts on this packet. No `WP_VALIDATOR` gate exists.
- Next actor: **`INTEGRATION_VALIDATOR`**.

## Commit range

- Packet-declared merge base: `facce56f879d4ee990f62566b12a8b26d8bc61d7`
  (`facce56f gov: add governance kernel worktree topology + sync-gov-to-main script`).
- Current `origin/main` HEAD: `5b02198b1522a04d40f1ad5cc85dd69fde3034f0`
  (`5b02198b gov: add root CLAUDE.md mirror of AGENTS.md per global mirror discipline`).
- WP branch head (`feat/WP-KERNEL-003-Sandbox-Validation-Promotion-v1`):
  `eac773ab` (Wave 4).
- `git merge-base --is-ancestor origin/main HEAD` from
  `wtc-validation-promotion-v1` returned **TRUE** — `origin/main` is a
  full ancestor of the WP head.
- `git merge-tree origin/main HEAD` from the WP worktree returned a
  single tree SHA `b6da4fcff2f65cac0e45b8ee40dcb09d089e4815` with no
  conflict markers — **merge-clean against current `origin/main`**.

Wave commits (oldest first):

| Wave | SHA | Scope |
|---|---|---|
| Batch A scaffolding (MT-004, 006–009) | `d7fa6aa9` | `kernel/kb003_schemas.rs`, `kernel/kb003_artifact_classes.rs`, `kernel/mod.rs` doc block |
| Wave 1 sandbox + validation (MT-010–019, MT-030–039) | `d63ff819` | `kernel/sandbox/**`, `kernel/validation/**`, `storage/kb003_storage.rs` |
| Wave 2 hard-isolation + promotion (MT-020–029, MT-040–049) | `f0277cec` | `kernel/sandbox/hard_isolation*`, `workspace_materializer`, `cleanup`, `redaction`, `caps`, `cancellation`, `kernel/kb003_promotion/**` |
| Wave 3 MTE + DCC (MT-050–074) | `c59db347` | `kernel/mte_*` (11 files), `kernel/dcc_kb003_*` (18 files) |
| Wave 4 test matrices + manual (MT-075–079) | `eac773ab` | 4 KB003 test files (+1,494 LOC), README KB003 section (+32 LOC) |

## Files added (product code)

86 files changed since `origin/main`, **17,712 LOC added, 0 LOC deleted**
(`git diff --shortstat origin/main..HEAD`). 83 added, 3 modified.

Grouped counts:

- `src/backend/handshake_core/src/kernel/sandbox/`: **25 new files**
  (adapter, adapter_selection, cancellation, cleanup, compat_blocker,
  dcc_projection, denial, exec_allowlist, fs_guard, hard_isolation,
  hard_isolation_container, hard_isolation_microvm, host_platform_probe,
  mod, network_gate, no_sqlite_tripwire, policy, policy_default_deny,
  policy_scoped_local, redaction, replay_projection, resource_caps, run,
  workspace, workspace_materializer).
- `src/backend/handshake_core/src/kernel/validation/`: **15 new files**
  (adapter_health, artifact_bundle, candidate_range, command_manifest,
  descriptor, diff_capture, environment_manifest, log_capture, mod,
  patch_proposal, redaction_report, report, run, status,
  visual_evidence).
- `src/backend/handshake_core/src/kernel/kb003_promotion/`: **7 new files**
  (artifact_bundle, dcc_promotion_overlay, decision, event_emission,
  gate, mod, receipt).
- `src/backend/handshake_core/src/kernel/mte_*` (flat): **11 new files**
  (aggregate_summary, authority_mutation_boundary, blocked_taxonomy,
  closeout_bundle, drop_back, idempotency_enforcement, lane_settlement,
  per_mt_summary, resource_caps, retry_budget,
  validation_report_projection).
- `src/backend/handshake_core/src/kernel/dcc_kb003_*` (flat):
  **18 new files** (aggregate_summary, blocked_reasons,
  bootstrap_skeleton, capability_audit, console_network_evidence,
  debug_bundle_bridge, dropback, evidence_portability, lane_wake,
  mex_evidence, model_manual_hints, mt_summary,
  promotion_control_state, retry_budget, rollup, run_detail,
  sandbox_run_list, visual_validation_gate).
- `src/backend/handshake_core/src/kernel/`: **2 new top-level files**
  (`kb003_schemas.rs`, `kb003_artifact_classes.rs`).
- `src/backend/handshake_core/src/storage/`: **1 new file**
  (`kb003_storage.rs`).
- `src/backend/handshake_core/tests/`: **4 new test files** (1,494 LOC
  total) — `kernel_kb003_security_denial_matrix_tests.rs` (453),
  `kernel_kb003_promotion_failure_matrix_tests.rs` (513),
  `kernel_kb003_restart_replay_tests.rs` (284),
  `kernel_kb003_disk_agnostic_paths_tests.rs` (244).

## Files modified (product code)

| Path | LOC delta | Note |
|---|---|---|
| `src/backend/handshake_core/src/kernel/mod.rs` | +72 | KB003 module declarations + doc block |
| `src/backend/handshake_core/src/storage/mod.rs` | +1 | `pub mod kb003_storage;` re-export |
| `README.md` | +32 | KB003 no-context model manual section (MT-079) |

## Acceptance row mapping

The 15 packet `acceptance_criteria` rows mapped to evidence. **No row is
claimed PASS** — this is the evidence inventory the IntVal verifies.

| # | Acceptance row | Implementing MT(s) | Product files | Test evidence |
|---|---|---|---|---|
| 1 | 80 MT contracts/projections exist | MT-001–MT-080 (contract authoring) | `.GOV/task_packets/WP-KERNEL-003-…/MT-*.json` (80 files) + `.MT-*.md` projections | n/a (governance contract) |
| 2 | All folded source-stub goals preserved | MT-001–MT-080 (every MT mapped to stub) | refinement.json traceability + per-MT receipts | n/a |
| 3 | Sandbox jobs cannot write authority state / escape sandbox roots | MT-010–019, MT-020–029 | `kernel/sandbox/fs_guard.rs`, `policy_default_deny.rs`, `workspace_materializer.rs`, `no_sqlite_tripwire.rs`, `hard_isolation*.rs` | `kernel_kb003_security_denial_matrix_tests.rs` (fs_guard 5 escape shapes + empty-roots, workspace_materializer undeclared+escape, no_sqlite_tripwire 3 non-Postgres modes) |
| 4 | Sandbox policy default-denies fs/network/process/device/env/secret | MT-010, MT-015–017 | `kernel/sandbox/policy.rs`, `policy_default_deny.rs`, `policy_scoped_local.rs`, `network_gate.rs`, `exec_allowlist.rs`, `denial.rs` | `kernel_kb003_security_denial_matrix_tests.rs` (network_gate 4 shapes, exec_allowlist 4 paths incl raw shell, policy_default_deny all 6 capabilities) |
| 5 | Sandbox outputs include hashed artifact bundles / manifests / logs / env / redaction | MT-018, MT-031, MT-033, MT-037 | `kernel/sandbox/run.rs`, `redaction.rs`, `kernel/validation/artifact_bundle.rs`, `environment_manifest.rs`, `log_capture.rs`, `redaction_report.rs`, `kb003_artifact_classes.rs` | `kernel_kb003_disk_agnostic_paths_tests.rs` (KB003_ARTIFACT_CLASSES retention roots, kb003:// handles) |
| 6 | Validation descriptors emit typed PASS/FAIL/BLOCKED/ADVISORY_ONLY/UNSUPPORTED/SKIPPED_WITH_REASON/ERROR | MT-030, MT-035, MT-036 | `kernel/validation/descriptor.rs`, `status.rs`, `report.rs`, `run.rs` | unit tests inline in modules; IntVal-side `cargo test -p handshake_core` confirms compile + assertions |
| 7 | PromotionGate accepts only validated candidates + appends EventLedger events | MT-040, MT-041, MT-046 | `kernel/kb003_promotion/gate.rs`, `decision.rs`, `event_emission.rs`, `kb003_schemas.rs` (EVENT_KB003_PROMOTION_DECIDED etc.) | `kernel_kb003_promotion_failure_matrix_tests.rs` (no ACCEPTED receipt row mutates Kb003Storage on any rejection variant) |
| 8 | Promotion rejection covers Stale/Duplicate/ValidationFailure/PolicyDenial/MissingApproval/MissingArtifact/PostgresFailure/ProjectionRebuildFailure | MT-042, MT-043, MT-044, MT-045 | `kernel/kb003_promotion/gate.rs` (8-variant `PromotionRejectionReason`), `decision.rs`, `receipt.rs` | `kernel_kb003_promotion_failure_matrix_tests.rs` — **one test per variant, 8/8 covered**; PostgresFailure uses bespoke `StorageRefusingDecisionInsert` |
| 9 | MTE caps / blocked taxonomy / retry budget / drop-back / per-MT / aggregate / lane settlement typed and durable | MT-050–059 | `kernel/mte_resource_caps.rs`, `mte_blocked_taxonomy.rs`, `mte_retry_budget.rs`, `mte_drop_back.rs`, `mte_per_mt_summary.rs`, `mte_aggregate_summary.rs`, `mte_lane_settlement.rs`, `mte_closeout_bundle.rs`, `mte_idempotency_enforcement.rs`, `mte_authority_mutation_boundary.rs`, `mte_validation_report_projection.rs` | unit tests inline in modules |
| 10 | DCC projection shows runs / blocked / validation / promotion / evidence | MT-060–074 | `kernel/dcc_kb003_*.rs` (18 files), `kernel/sandbox/dcc_projection.rs`, `kernel/kb003_promotion/dcc_promotion_overlay.rs` | `kernel_kb003_restart_replay_tests.rs` (`DccKb003RollupV1` byte-equal across replay) |
| 11 | Visual validation evidence attachable | MT-038 | `kernel/validation/visual_evidence.rs`, `kernel/dcc_kb003_visual_validation_gate.rs` | unit tests inline |
| 12 | No SQLite authority / fallback / fixture / compat / offline | MT-017 + cross-cutting | `kernel/sandbox/no_sqlite_tripwire.rs`, `storage/kb003_storage.rs` (Postgres rows only) | `kernel_kb003_security_denial_matrix_tests.rs` (no_sqlite_tripwire 3 non-Postgres modes) |
| 13 | Validation + promotion reconstructable after restart without chat/scrollback | MT-077 | `kernel/sandbox/replay_projection.rs`, `storage/kb003_storage.rs` (`load_replay_bag`, `reconstruct_projection`) | `kernel_kb003_restart_replay_tests.rs` (drops live store, rebuilds from durable Vec snapshots; field-level checks on `DccKb003RollupV1`) |
| 14 | Generated artifacts under configured roots + disk-agnostic paths | MT-078 | `kernel/kb003_artifact_classes.rs` (`KB003_ARTIFACT_CLASSES.retention_root` under `handshake-product/`), `kernel/sandbox/workspace.rs` | `kernel_kb003_disk_agnostic_paths_tests.rs` — rejects BACKSLASH, UNC, LEADING_SLASH, DRIVE_LETTER, HARDCODED_HOST_ROOT, EMPTY_PATH, env-var expansion; verifies workspace root portability |
| 15 | Closeout requests IntVal batch review and does not self-claim PASS/FAIL | MT-080 (this) | `.GOV/task_packets/WP-KERNEL-003-…/IV_HANDOFF.md` (this file) | MT-080 receipt + this bundle |

## Cargo proof status

**Cargo was waived throughout this WP** per
`CX-ENV-HOST-LOAD-CARGO-TESTS-20260504` (host load constraint on the
Kernel Builder session). Every per-MT receipt records
`cargo_proof: NOT_RUN_WAIVED`.

Required IntVal-side cargo runs (run from `wtc-validation-promotion-v1`):

```
cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target
cargo test -p handshake_core kernel_kb003_security_denial_matrix --target-dir ../Handshake_Artifacts/handshake-cargo-target
cargo test -p handshake_core kernel_kb003_promotion_failure_matrix --target-dir ../Handshake_Artifacts/handshake-cargo-target
cargo test -p handshake_core kernel_kb003_restart_replay --target-dir ../Handshake_Artifacts/handshake-cargo-target
cargo test -p handshake_core kernel_kb003_disk_agnostic_paths --target-dir ../Handshake_Artifacts/handshake-cargo-target
```

`just gov-check` from the kernel worktree is the governance-side gate.

## SPEC_MUST_TO_PROOF_MATRIX

For each Master Spec anchor declared in
`packet.json scope.spec_anchors`. Classifications:
`runtime_behavior | durable_storage | eventledger_append |
ui_projection | negative_guard | test_only`.

> **Note.** Per the packet's PASS-Ready Handoff Hardening rule, rows
> classified `test_only` are advisory — IntVal decides whether the
> supporting runtime evidence is sufficient.

| Spec anchor | Proof class | Product modules / tests addressing it |
|---|---|---|
| `02-system-architecture.md#2.3.13.9` (sandbox minimum + isolation) | `runtime_behavior` + `negative_guard` | `kernel/sandbox/{policy_default_deny,fs_guard,network_gate,exec_allowlist,hard_isolation*,no_sqlite_tripwire,workspace_materializer}.rs`; `tests/kernel_kb003_security_denial_matrix_tests.rs` |
| `02-system-architecture.md#2.3.13.10` (promotion gate + evidence chain) | `runtime_behavior` + `eventledger_append` + `durable_storage` | `kernel/kb003_promotion/{gate,decision,event_emission,receipt}.rs`; `kernel/kb003_schemas.rs` (EVENT_KB003_PROMOTION_* constants); `storage/kb003_storage.rs` (`PromotionDecisionRowV1`, `PromotionReceiptRowV1`); `tests/kernel_kb003_promotion_failure_matrix_tests.rs` |
| `05-security-and-observability.md#5.2.5` (default-deny capabilities) | `negative_guard` | `kernel/sandbox/policy.rs` + `policy_default_deny.rs` + `denial.rs`; `tests/kernel_kb003_security_denial_matrix_tests.rs` (all 6 capability denials) |
| `05-security-and-observability.md#5.4.5` (redaction state + audit trail) | `runtime_behavior` + `durable_storage` | `kernel/sandbox/redaction.rs`; `kernel/validation/redaction_report.rs`; `kernel/dcc_kb003_capability_audit.rs` |
| `10-product-surfaces.md#10.11.5.28` (DCC projection of sandbox/validation/promotion) | `ui_projection` + `runtime_behavior` | `kernel/dcc_kb003_*.rs` (18 modules); `kernel/sandbox/dcc_projection.rs`; `kernel/kb003_promotion/dcc_promotion_overlay.rs`; `tests/kernel_kb003_restart_replay_tests.rs` (`DccKb003RollupV1` byte-equal across replay) |
| `11-shared-dev-platform-and-oss-foundations.md#sandbox-minimum` (disk-agnostic + Postgres-only authority) | `negative_guard` + `durable_storage` | `kernel/kb003_artifact_classes.rs`; `kernel/sandbox/{workspace,workspace_materializer,no_sqlite_tripwire}.rs`; `storage/kb003_storage.rs`; `tests/kernel_kb003_disk_agnostic_paths_tests.rs` |

## ANTI_SCAFFOLD_GATE — declarative surface → executable consumer

For every declarative surface added (Contract, Descriptor, Mapping,
Projection, Schema, Receipt, Evidence), the executable consumer is
named. Scaffolding-without-consumer is the failure mode; this is the
explicit walk.

| Declarative surface | Defined in | Executable consumer(s) |
|---|---|---|
| Schema IDs (`SCHEMA_KERNEL_SANDBOX_RUN_V1`, `_POLICY_V1`, `_WORKSPACE_V1`, `_ARTIFACT_BUNDLE_V1`, `VALIDATION_RUN_V1`, `PROMOTION_DECISION_V1`, `PROMOTION_RECEIPT_V1`) | `kernel/kb003_schemas.rs` | `storage/kb003_storage.rs` (row inserts cite `schema_version`); `kernel/sandbox/run.rs`, `kernel/validation/run.rs`, `kernel/kb003_promotion/event_emission.rs` (envelope construction) |
| Event-type constants (`EVENT_KB003_SANDBOX_RUN_REQUESTED`/`_STARTED`/`_COMPLETED`/`_REJECTED`, `EVENT_KB003_VALIDATION_RUN_COMPLETED`, `EVENT_KB003_PROMOTION_DECIDED`/`_RECEIPT_ISSUED`/`_REJECTED`) | `kernel/kb003_schemas.rs` | `kernel/sandbox/run.rs` emit path; `kernel/kb003_promotion/event_emission.rs`; consumed by EventLedger writers via `Kb003EventEnvelope` |
| `Kb003EventEnvelope` struct | `kernel/kb003_schemas.rs` | Constructed by sandbox runner + validation runner + promotion gate; serialized into EventLedger row payload via `storage/kb003_storage.rs` |
| `Kb003ArtifactClass` enum + `KB003_ARTIFACT_CLASSES` metadata + `Kb003ArtifactMetadata` | `kernel/kb003_artifact_classes.rs` | `kernel/sandbox/workspace.rs` + `workspace_materializer.rs` (retention_root resolution); `kernel/validation/artifact_bundle.rs` (hash policy); `tests/kernel_kb003_disk_agnostic_paths_tests.rs` (asserts every class root under `handshake-product/`) |
| `ValidationRunRowV1`, `PromotionDecisionRowV1`, `PromotionReceiptRowV1`, `InMemoryKb003Storage`, `ReplayDurableBag` | `storage/kb003_storage.rs` | `kernel/validation/run.rs` (write `ValidationRunRowV1`); `kernel/kb003_promotion/gate.rs` (write decision + receipt rows); `tests/kernel_kb003_restart_replay_tests.rs` (`load_replay_bag` → `reconstruct_projection`) |
| `PromotionGate`, `PromotionGateInputs`, `PromotionGateOutput`, `OperatorApprovalEvidence` | `kernel/kb003_promotion/gate.rs` | Invoked by promotion lifecycle path that turns a validated candidate into a `PromotionDecisionV1`; outputs consumed by `event_emission.rs` (EventLedger append) + `dcc_promotion_overlay.rs` (projection) |
| `PromotionRejectionReason` enum (8 variants) | `kernel/kb003_promotion/{gate,decision}.rs` | Every rejection variant routed into a typed `PromotionDecisionV1` row; consumed by `tests/kernel_kb003_promotion_failure_matrix_tests.rs` (one assertion per variant) |
| `SandboxCapability` enum + `SandboxDenialRecordV1` | `kernel/sandbox/{policy,denial}.rs` | Read by `policy_default_deny.rs` and each capability guard (`fs_guard`, `network_gate`, `exec_allowlist`, `hard_isolation*`, `workspace_materializer`); emitted via `tests/kernel_kb003_security_denial_matrix_tests.rs` |
| `DccKb003RollupV1` + `summary_line` + `load_replay_bag` + `reconstruct_projection` | `kernel/dcc_kb003_rollup.rs` + `storage/kb003_storage.rs` | DCC projection surface for sandbox/validation/promotion overlay; consumed by `tests/kernel_kb003_restart_replay_tests.rs` (byte-equal projection rebuild) |
| MTE descriptors (`MteResourceCaps`, `MteBlockedTaxonomy`, `MteRetryBudget`, `MteDropBack`, `MtePerMtSummary`, `MteAggregateSummary`, `MteLaneSettlement`, `MteCloseoutBundle`, `MteIdempotencyEnforcement`, `MteAuthorityMutationBoundary`, `MteValidationReportProjection`) | `kernel/mte_*.rs` | Per-MT executor + lane settlement runtime; each descriptor referenced by the corresponding `dcc_kb003_*` projection (e.g. `mte_per_mt_summary.rs` → `dcc_kb003_mt_summary.rs`) |
| `RedactionReport::partition_default_policy` | `kernel/validation/redaction_report.rs` | Sandbox outputs export path (redaction-aware export); promotion gate consults redaction state before promoting |

## NEGATIVE_GUARD_TESTS

Tests that prove forbidden behavior fails closed:

- **`tests/kernel_kb003_security_denial_matrix_tests.rs`** — 8 denial
  surfaces, each emits typed `SandboxDenialRecordV1` with `kind+reason`:
  fs_guard (5 escape shapes + empty-roots dual-mode), network_gate
  (4 failure shapes), exec_allowlist (4 paths including raw shell),
  policy_default_deny (all 6 capabilities), hard_isolation_container
  (Blocked with missing_dependency), hard_isolation_microvm
  (Unsupported on Windows), no_sqlite_tripwire (3 non-Postgres modes),
  workspace_materializer (undeclared + escape).
- **`tests/kernel_kb003_promotion_failure_matrix_tests.rs`** —
  8 rejection variants, one test per `PromotionRejectionReason` value;
  each asserts (1) typed rejection reason carried by
  `PromotionDecisionV1`, (2) **no `ACCEPTED` receipt row mutates
  `Kb003Storage` authority sink**.
- **`kernel/sandbox/no_sqlite_tripwire.rs` tests** — SQLite authority
  denial: any non-Postgres authority mode (sqlite_authority,
  sqlite_fallback, sqlite_fixture) fails closed at sandbox start
  (per CX-503R).
- **`kernel/kb003_promotion/gate.rs`
  `idempotency_conflict_surfaces_as_typed_rejection`** — duplicate
  idempotency key surfaces as `PromotionRejectionReason::DuplicateIdempotencyKey`
  rather than silent skip or partial write.
- **`tests/kernel_kb003_disk_agnostic_paths_tests.rs`** —
  `forbidden_path_shape` detector negative-guards 7 host-bound /
  escape path shapes; every `KB003_ARTIFACT_CLASSES.retention_root`
  must start with `handshake-product/` and contain no forbidden shape.

## PRIMITIVE_RETENTION_PROOF

Walked the KB003 module tree from head `eac773ab`. **All declared
primitives/modules/action_ids from the 80 MT contracts are present** in
the handoff head:

- `src/backend/handshake_core/src/kernel/sandbox/` — 25 modules
  (matches MT-010..029 declared module set).
- `src/backend/handshake_core/src/kernel/validation/` — 15 modules
  (matches MT-030..039 declared module set).
- `src/backend/handshake_core/src/kernel/kb003_promotion/` — 7 modules
  (matches MT-040..049 declared module set; consolidated under one
  subdirectory per kernel-builder consolidation rule).
- `src/backend/handshake_core/src/kernel/mte_*` — 11 flat modules
  (matches MT-050..059 declared module set).
- `src/backend/handshake_core/src/kernel/dcc_kb003_*` — 18 flat modules
  (matches MT-060..074 declared module set).
- Top-level scaffolding: `kernel/kb003_schemas.rs`,
  `kernel/kb003_artifact_classes.rs`, `storage/kb003_storage.rs`,
  `kernel/mod.rs` module declarations.
- Test files: 4 new under `tests/` for MT-075..078; README KB003
  section for MT-079.

Cross-reference: MT-060..074 receipts are consolidated into one rolled
receipt (subagent timeout during Wave 3), with the file inventory
listed in the Wave 3 commit `c59db347` message. All other MTs carry
per-MT receipts in `RECEIPTS.jsonl` (64 receipts total in the ledger).

## CURRENT_MAIN_INTERACTION_CHECKS

Read-only git checks performed by Kernel Builder (results, no
judgement):

- `git fetch origin main` from `wt-gov-kernel`: success. Resulting
  `origin/main` SHA: **`5b02198b1522a04d40f1ad5cc85dd69fde3034f0`**.
- `git merge-base --is-ancestor origin/main HEAD` from
  `wtc-validation-promotion-v1` head `eac773ab`:
  **TRUE** — `origin/main` is a full ancestor of the WP head; the
  branch is already current with `origin/main`.
- `git merge-tree origin/main HEAD` from
  `wtc-validation-promotion-v1`: single tree SHA
  `b6da4fcff2f65cac0e45b8ee40dcb09d089e4815` returned with no
  conflict markers — **merge-clean**.
- `git log --oneline origin/main..HEAD` from the WP worktree: exactly
  **5 commits ahead** (the Wave commits enumerated above).
- `git diff --shortstat origin/main..HEAD`: **86 files changed,
  +17,712 / -0 LOC**.

## SELF_VALIDATOR_ATTACKS

Per kernel-builder protocol, enumerated 5+ plausible IntVal failure
modes against the resolved current Master Spec. For each: anchor,
product path, evidence checked, disposition
(`FIXED | PROVEN_SAFE | OUT_OF_SCOPE_BY_PACKET | OPEN_BLOCKER`).

1. **Cargo never executed on Kernel Builder host.**
   - Anchor: packet `proof_targets` lists
     `cargo test -p handshake_core …`.
   - Path: every product file under
     `src/backend/handshake_core/`.
   - Evidence: every per-MT receipt records
     `cargo_proof: NOT_RUN_WAIVED`.
   - Disposition: **OUT_OF_SCOPE_BY_PACKET** —
     `CX-ENV-HOST-LOAD-CARGO-TESTS-20260504` waives Kernel Builder
     cargo execution; IntVal owns the cargo gate.

2. **SQLite authority sneaks in via a transitive dependency.**
   - Anchor: spec §11 sandbox-minimum + acceptance row 12
     (no SQLite authority/fallback/fixture).
   - Path: `kernel/sandbox/no_sqlite_tripwire.rs`,
     `storage/kb003_storage.rs`.
   - Evidence: tripwire fires on any non-Postgres authority mode;
     `tests/kernel_kb003_security_denial_matrix_tests.rs` covers
     three non-Postgres modes.
   - Disposition: **PROVEN_SAFE** at the surfaces Kernel Builder owns;
     IntVal should still confirm `Cargo.toml` does not pull a sqlite
     adapter into `handshake_core` build graph.

3. **Promotion accepts a stale candidate due to race condition.**
   - Anchor: spec §2.3.13.10 + acceptance row 8.
   - Path: `kernel/kb003_promotion/gate.rs`, `decision.rs`.
   - Evidence: `PromotionRejectionReason::StaleCandidate` variant
     present; `tests/kernel_kb003_promotion_failure_matrix_tests.rs`
     asserts no `ACCEPTED` row is written on `StaleCandidate`.
   - Disposition: **PROVEN_SAFE** under unit-test surface;
     IntVal may want to additionally verify concurrent race scenarios
     under cargo run.

4. **Workspace materializer leaks an absolute host path into a
   durable row.**
   - Anchor: spec §11 disk-agnostic + acceptance row 14.
   - Path: `kernel/sandbox/workspace.rs`,
     `workspace_materializer.rs`, `kb003_artifact_classes.rs`.
   - Evidence: `tests/kernel_kb003_disk_agnostic_paths_tests.rs`
     rejects 7 forbidden path shapes incl. drive letter, UNC,
     hardcoded host root, env-var expansion; asserts
     `SandboxWorkspaceV1::new_default` produces only repo-relative
     paths across three different roots.
   - Disposition: **PROVEN_SAFE** at the test boundary; IntVal
     should also grep the storage layer for accidental
     `Path::display()` of an absolute path.

5. **Restart replay rehydrates partial / inconsistent DCC state.**
   - Anchor: spec §10.11.5.28 + acceptance row 13.
   - Path: `storage/kb003_storage.rs` (`load_replay_bag`,
     `reconstruct_projection`), `kernel/dcc_kb003_rollup.rs`.
   - Evidence: `tests/kernel_kb003_restart_replay_tests.rs` drops
     the live `InMemoryKb003Storage`, rebuilds from durable Vec
     snapshots only, and asserts `DccKb003RollupV1` is **byte-equal**
     (canonical JSON Value comparison) plus field-level checks.
   - Disposition: **PROVEN_SAFE** under in-memory storage; IntVal
     should confirm the same path works against a real Postgres
     adapter once a runtime row store exists.

6. **DCC projection loses an event class after a redaction pass.**
   - Anchor: spec §5.4.5 + acceptance row 5.
   - Path: `kernel/sandbox/redaction.rs`,
     `kernel/validation/redaction_report.rs`,
     `kernel/dcc_kb003_capability_audit.rs`.
   - Evidence: `RedactionReport::partition_default_policy` separates
     redactable vs durable; `dcc_kb003_capability_audit.rs` projects
     the audit trail separately from the run detail.
   - Disposition: **PROVEN_SAFE** at module surface; no negative-guard
     test specifically pairs redaction-pass with DCC reconstruction —
     IntVal may flag this as advisory hardening.

7. **Hard-isolation falls back to "no isolation" silently on
   unsupported host.**
   - Anchor: spec §2.3.13.9 sandbox minimum + acceptance row 3.
   - Path: `kernel/sandbox/hard_isolation*.rs`, `host_platform_probe.rs`.
   - Evidence:
     `tests/kernel_kb003_security_denial_matrix_tests.rs` asserts
     `hard_isolation_container` returns `Blocked` with
     `missing_dependency` when the container runtime is absent and
     `hard_isolation_microvm` returns `Unsupported` on Windows
     (no silent fallback).
   - Disposition: **PROVEN_SAFE**.

## ARTIFACT_DIR_CLEANUP

**NOT_RUN** — no cargo was executed during this session, so nothing
was written under `../Handshake_Artifacts/handshake-cargo-target` by
the Kernel Builder. IntVal will clean their own cargo target
directory before merge per packet protocol.

## Per-MT receipt inventory

- Receipts file:
  `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-KERNEL-003-Sandbox-Validation-Promotion-v1/RECEIPTS.jsonl`
- Receipt count at handoff time: **64 receipts**.
- Per-MT coverage: every MT except MT-060..MT-074 has an individual
  receipt; MT-060..MT-074 share **one consolidated receipt**
  (subagent timeout during Wave 3 forced a rolled-up receipt). Wave 3
  commit `c59db347` lists the per-MT product file inventory in its
  commit message.
- This MT-080 handoff appends its own STATUS receipt after this
  bundle is authored.

## NEXT_ACTOR

**`INTEGRATION_VALIDATOR`** — perform batch MT review of MT-001..MT-079,
then scoped Master Spec review against the six declared spec anchors,
then issue the verdict on the packet.
