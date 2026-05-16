---
file_id: integration-validation-appendage-20260516-mt-rerun-10
file_kind: validation_appendage
updated_at: 2026-05-16T17:30:00Z
wp_id: WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
owner: INTEGRATION_VALIDATOR
candidate_worktree: D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-preuse-hardening-v1
candidate_branch: feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1
candidate_head_sha: 208e5503
candidate_state: COMMITTED_CLEAN
baseline_main_sha: e11ba597
mt_batch_verdict: PASS_WITH_HARDENING_NOTES
whole_wp_master_spec_validation: PROCEEDING
---

<topic id="validation-context" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T17:30:00Z" ingestable="true" summary="Integration Validator rerun 10 after Kernel Builder committed remediation 208e5503.">

Handshake (Product): reviewed the Kernel Builder committed remediation `208e5503 remediate kernel mt043 mt045 mt049` on top of `e09fa152` in `wtc-preuse-hardening-v1`. All three previously-failing MTs (MT-043, MT-045, MT-049) now meet their acceptance text. Test-rigor and wiring concerns are recorded but do not block MT acceptance under the operator-confirmed waiver.

Repo Governance: Operator waived normal WP Validator workflow and governance paperwork during this remediation cycle. Proceeding to whole-WP code-vs-current-Master-Spec validation. No merge, sync, staging, commit, or push has been performed.

CANDIDATE_STATE:
- Worktree clean, on `feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1`.
- `git rev-list --left-right --count main...HEAD` = `0 6`.
- `git diff --check main...HEAD`: clean.
- `git merge-base --is-ancestor main HEAD`: TRUE.
- HEAD: `208e5503c9b11c5d8cd0548728582f3b27766b6b`.

MECHANICAL_INTERVENTION_CLASSIFICATION:
- Runtime route drift: CLOSED for MT-043; announce-back and cascade-cancel now derive from explicit Flight Recorder events and explicit capability grants.
- Scope/implementation drift: CLOSED for MT-045; real Playwright-driven capture adapter and `handshake` CLI binary now exist; DOM-aware selectors for full-app, panel, and module targets.
- Proof/receipt drift: CLOSED for MT-049; per-command receipt artifacts now present on disk with candidate SHA, exit codes, stdout/stderr refs, and blocker JSON; `gov-check` exit 1 produces concrete blocker proof.
- Documentation/protocol drift: N/A under operator waiver.
- Scope/worktree drift: CLOSED; remediation is a single named commit, not dirty working-tree state.

</topic>

<topic id="checks-run" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T17:30:00Z" ingestable="true" summary="Checks and evidence sources for rerun 10.">

CHECKS_RUN:
- Git topology: status, log, rev-list, diff --check, merge-base on `wtc-preuse-hardening-v1` HEAD vs `main`.
- `git show --stat 208e5503`: 15 files, +2049/-64.
- Targeted source review of:
  - `src/backend/handshake_core/src/api/kernel.rs` (announce-back, cascade-cancel, screenshot route registration)
  - `src/backend/handshake_core/src/kernel/session_spawn_tree_dcc.rs`
  - `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs`
  - `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs`
  - `src/backend/handshake_core/src/kernel/action_catalog.rs`
  - `src/backend/handshake_core/src/bin/handshake.rs` (new CLI binary)
  - `app/scripts/handshake-screenshot-capture.mjs` (new Playwright adapter)
  - `app/src/lib/api.ts`, `app/src/lib/api.test.ts`, `app/src/components/KernelDccProjectionView.tsx`
  - `src/backend/handshake_core/tests/kernel_mechanical_contract_generation_tests.rs`
  - `src/backend/handshake_core/tests/kernel_product_screenshot_capture_tests.rs`
  - `justfile` (recipe wiring for gov-check, build-order-sync, task-packet-stub-contracts)
- Direct inspection of `../Handshake_Artifacts/handshake-product/command-receipts/` confirming receipts for all 3 commands bind candidate SHA `208e5503`.
- 3 parallel read-only adversarial subagent reviews (one per MT).

TEST_VERDICT: ACCEPTANCE_MET_WITH_TEST_GAPS.

Tests on the candidate branch were not re-executed by this validator (cargo build/test out of scope for read-only MT review). Kernel Builder-reported test passes are accepted as input; remaining test-rigor gaps are recorded as hardening concerns, not acceptance blockers.

</topic>

<topic id="mt-findings" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T17:30:00Z" ingestable="true" summary="MT-level findings after rerun-10 review.">

MT_BATCH_VERDICT: PASS (with hardening notes recorded for forward work).

WHOLE_WP_MASTER_SPEC_VALIDATION: PROCEEDING (separate output in this validator session).

MT-043 VERDICT: PASS.

Finding: announce-back badges are now extracted from `FlightRecorderEventType::SessionSpawnAnnounceBack` events; cascade-cancel support is now backed by explicit `session_has_explicit_cascade_cancel_capability` checks AND `FlightRecorderEventType::SessionCascadeCancel` events. The rerun-9 synthesis gap is closed in code.

Evidence:
- `src/backend/handshake_core/src/api/kernel.rs:386-391` — explicit capability check drives `evidence.cascade_cancel_session_ids`.
- `src/backend/handshake_core/src/api/kernel.rs:407-417` — `announce_back_badge_from_event` extracts badges from FR events scoped to known session ids.
- `src/backend/handshake_core/src/api/kernel.rs:421-435` — `SessionCascadeCancel` FR events also populate cascade-cancel evidence.
- `src/backend/handshake_core/src/api/kernel.rs:463-490` — `announce_back_badge_from_event` and `session_has_explicit_cascade_cancel_capability` helpers.

Hardening note (non-blocking): test coverage in `kernel_dcc`-related tests still injects `SessionSpawnRuntimeEvidence` directly with empty `capability_grants` rather than driving evidence through Flight Recorder events. A regression in the new extraction helpers would not be caught. Recommend adding tests that (a) feed `SessionSpawnAnnounceBack` events and assert badges appear, (b) set `capability_grants` and assert cascade-cancel surfaces, (c) feed `SessionCascadeCancel` FR events and assert cascade-cancel surfaces.

MT-045 VERDICT: PASS.

Finding: a real capture adapter now exists. The `handshake` CLI binary executes `screenshot capture`, spawning a Node Playwright adapter that performs DOM-aware screenshots against an HTTP source URL; full-app, panel, and module targets resolve to actual selectors. The rerun-9 "no real capture" gap is closed.

Evidence:
- `src/backend/handshake_core/src/bin/handshake.rs` (new, +349) — `[[bin]] name = "handshake"`; CLI subcommands `screenshot capture` and `command receipt run`.
- `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs:495-560` (`capture_product_screenshot_from_browser_adapter`) — spawns Node adapter with `Command::new(node_binary)`, reads the resulting PNG from disk, validates with `image::load_from_memory_with_format`.
- `app/scripts/handshake-screenshot-capture.mjs:80-100` (`selectorFor`) — full-app returns null (Playwright `fullPage: true`); panel returns `[aria-labelledby="..."]`; module returns `[data-testid="..."]` / `[data-stable-id="..."]`.
- `src/backend/handshake_core/src/kernel/product_screenshot_capture.rs:1056-1062` (`target_matches_scope`) — enforces `app://`, `panel://`, `module://` prefixes per scope.
- `src/backend/handshake_core/src/api/kernel.rs:592-593` — `/kernel/product_screenshot_capture/execute` route is registered.

Hardening notes (non-blocking):
- No end-to-end integration test invokes the Node Playwright adapter against a live test page. Recommend a smoke test that spawns a local HTML fixture, runs the CLI, and asserts a non-empty PNG with metadata width/height matching the captured image.
- `capture_product_screenshot_from_browser_adapter` does not pre-check Node/Playwright availability; failures surface as opaque `AdapterFailed`. Recommend an explicit pre-flight or actionable error.
- `app/scripts/handshake-screenshot-capture.mjs:80-97` selector branches for non-`kernel-dcc` panel/module refs are uncovered by tests.

MT-049 VERDICT: PASS.

Finding: per-command receipt artifacts for `just task-packet-stub-contracts --all`, `just build-order-sync`, and `just gov-check` exist on disk under `../Handshake_Artifacts/handshake-product/command-receipts/`, bind candidate SHA `208e5503`, and record exit codes, stdout/stderr artifact refs, and (for `gov-check`) concrete blocker refs. The rerun-9 "no current-candidate command receipt artifacts" gap is closed. The acceptance text "pass or produce a concrete blocker" is met: `gov-check` exit 1 with `gov-check.blockers.json` is the concrete-blocker path.

Evidence:
- `../Handshake_Artifacts/handshake-product/command-receipts/gov-check.json` — schema `hsk.current_candidate_command_receipt@1`, `candidate_sha: 208e5503...`, `actual_exit_code: 1`, `blocker_refs: ["blocker://current-candidate-command/gov-check/exit-code-1"]`, stdout/stderr SHA256 + refs.
- Sibling artifacts: `build-order-sync.json`, `task-packet-stub-contracts-all.json` plus `.stdout.txt` / `.stderr.txt` / `.blockers.json` files.
- `src/backend/handshake_core/src/bin/handshake.rs:36-38, 99-131, 263-325` — `handshake command receipt run` subcommand writes these receipts.
- `src/backend/handshake_core/src/kernel/mechanical_contract_generation.rs` (+309) — durable schema and writer for `hsk.current_candidate_command_receipt@1`.

Hardening note (non-blocking): the `justfile` recipes for `gov-check`, `build-order-sync`, and `task-packet-stub-contracts` invoke the underlying Node scripts directly; they do NOT wrap into `handshake command receipt run`. A fresh validator running `just gov-check` will not auto-produce a receipt. The current receipts were produced by separate CLI invocations against the candidate SHA. Recommend wrapping the three recipes so receipts are produced automatically on every invocation; this is a maintainability/reproducibility improvement, not an acceptance blocker.

</topic>

<topic id="next-step" status="closed" version="1" wp="WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1" owner="INTEGRATION_VALIDATOR" updated_at="2026-05-16T17:30:00Z" ingestable="true" summary="MT batch PASS; proceeding to whole-WP code-vs-Master-Spec validation.">

Next: whole-WP coding-vs-current-Master-Spec validation across all 61 MTs and the WP refinement. Output will land as a separate verdict, not as a microtask appendage.

Per operator instruction in this session: governance paperwork (validator-gate-append, validator-gate-commit, task-board sync, terminal_closeout_record, validator-report-structure machinery) is waived during the MT batch phase. Sync of governance documentation will be performed only after whole-WP PASS and explicit operator green-light.

</topic>
