# AUDIT-20260505-POSTGRES-PRIMARY-ACP-SWARM-HARDENING

## Metadata

- AUDIT_ID: AUDIT-20260505-POSTGRES-PRIMARY-ACP-SWARM-HARDENING
- STATUS: ACTIVE
- CREATED_AT: 2026-05-05T17:55:00Z
- OWNER: ORCHESTRATOR
- GOVERNANCE_ITEM: RGF-281
- SCOPE: Repo Governance
- PRODUCT_WP_CONTEXT: WP-1-Postgres-Primary-Control-Plane-Foundation-v1

## Driver

The Operator started `WP-1-Postgres-Primary-Control-Plane-Foundation-v1` as an orchestrator-managed ACP workflow, declared GPT-5.5 extra-high for all governed roles except WP Validator on Claude Opus 4.7 extra thinking, warned that high-volume scripts and host load may be active, and asked the Orchestrator to babysit ACP while hardening the workflow for future swarm-parallel work packets.

## Scope

In scope:
- ACP/session-control runtime checks for accepted/running/queued states under heavy load.
- Activation Manager handoff and duplicate-stub/refinement responsibility boundaries.
- Packet/role model-profile self-checking for GPT-5.5 and Claude Opus 4.7 profile IDs.
- Repomem coverage debt visibility during long-running orchestrator-managed sessions.
- Fallback/repair guidance for stale, stalled, queued, or busy ACP sessions.

Out of scope:
- Product code under `src/`, `app/`, or `tests/`.
- Master Spec text or PostgreSQL product implementation.
- Replacing WP Validator or Integration Validator technical authority.
- Destructive cleanup or worktree deletion.

## Initial Runtime Evidence

- Activation Manager `START_SESSION` for `WP-1-Postgres-Primary-Control-Plane-Foundation-v1` was accepted as running under `HANDSHAKE_ACP_BROKER`.
- The follow-up stub instruction was accepted as queued behind the active Activation Manager run.
- The operator override assigning stub creation to Orchestrator was also accepted as queued.
- `session-registry-status` surfaced repomem coverage debt and token-ledger drift as diagnostic/runtime truth while the role session was still starting.
- The historical launch batch mode already reflected plugin instability and CLI escalation context, so heavy-load checks must distinguish current-session health from historical batch mode.

## Candidate Hardening Targets

- Accepted/queued state duplicate suppression: prevent repeated sends when ACP already accepted work.
- Activation Manager handoff split: make refinement/spec-enrichment ownership mechanically distinct from Orchestrator-created stub backlog work.
- Model-profile enforcement: fail fast when packet/session profile IDs drift from operator-declared role profiles.
- Repomem coverage debt: keep debt visible without treating startup-phase debt as final session failure.
- Heavy-load fallback checks: distinguish busy, stalled, timed out, accepted-running, and terminal states in operator-facing commands.
- Parallel WP monitoring: make multi-WP status summaries show next expected actor/session, queued commands, and stale-age in one primary surface.

## Implemented Slice 2026-05-05

- `worktree-concurrency-check.mjs` now treats active `ACTIVATION_MANAGER`, `ORCHESTRATOR`, and `CLASSIC_ORCHESTRATOR` sessions as prelaunch workflow-authority lanes that do not by themselves require coder/WP-validator worktree mappings before packet activation.
- Task Board `IN_PROGRESS` entries still require dedicated WP worktree mappings, so executable product work keeps the one-WP/one-worktree guard.
- Focused tests cover role-level exclusion and the rule that an `IN_PROGRESS` Task Board entry for the same WP re-enables the dedicated-worktree requirement.

## Implemented Slice 2026-05-05B

- `ACTIVATION_READINESS` now carries `GENERATED_AT_UTC`, `STATE_SOURCE`, and `READY_FOR_DOWNSTREAM_LAUNCH` fields so stale handoff files are mechanically visible.
- `orchestrator-next` now gives Activation Manager readiness repair precedence over downstream runtime relay and suppresses bootstrap `VALIDATOR_KICKOFF` residue when no Coder/WP Validator session exists yet.
- Relay escalation now classifies prelaunch bootstrap validator-kickoff state as `PRELAUNCH_NOT_APPLICABLE` instead of recommending `orchestrator-steer-next`.
- The live PostgreSQL-primary WP readiness artifact was regenerated after `build-order-sync`; it now reports `READY_FOR_ORCHESTRATOR_REVIEW` with all activation checks passing.
- Master Spec v02.182 health was checked against the main-worktree v02.181 backup: the diff is bounded to the approved PostgreSQL-primary enrichment, spec bundle/EOF checks pass, conflict markers are absent, and the SHA1 matches the signed refinement.

## Implemented Slice 2026-05-05C

- `phase-check STARTUP` now resolves Coder pre-work/post-work and confinement checks through the packet-declared `LOCAL_WORKTREE_DIR`, so Orchestrator-launched gates from `wt-gov-kernel` do not falsely inspect the governance kernel worktree.
- Startup mesh checks now defer prelaunch Coder failure when `active-lane-brief` truthfully reports no launched Coder/WP Validator sessions yet.
- `wp-communication-health-check` now merges live non-terminal role sessions from the session registry into `RUNTIME_STATUS.active_role_sessions`, and publishes both canonical `ROLE:WP-ID` and lower-case actor aliases for startup mesh compatibility.
- `wp-relay-escalation` gives newly moved governed target sessions a receipt grace window before recommending orchestrator steering.
- `wp-token-usage-lib` excludes in-flight raw command outputs with no `turn.completed` from hard drift classification while still reporting pending raw command ids.
- `orchestrator-next` now detects already active downstream Coder/WP Validator sessions and projects steering/status commands instead of relaunching them.

## Implemented Slice 2026-05-05D

- `repomem` now stores role/WP-scoped session markers under `REPOMEM_SESSIONS/` while preserving the legacy current marker for compatibility.
- `repomem open` auto-closes only the same role/WP lane; concurrent Orchestrator, Coder, WP Validator, and Integration Validator memory sessions are preserved for parallel ACP workflows.
- `repomem gate`, current-session lookup, close, context, and checkpoint commands infer role/WP scope from `--role`, `--wp`, `HANDSHAKE_ROLE`, and `WP_ID` so governed sessions can find their own marker without extra prompt tokens.
- `wp-relay-escalation` gives a fresh route its own receipt grace window when old runtime `heartbeat_due_at` / `stale_after` clocks predate the new notification route.
- Live recovery proof: the WP Validator startup mesh passed, `VALIDATOR_KICKOFF` was published with correlation id `review:WP-1-Postgres-Primary-Control-Plane-Foundation-v1:validator_kickoff:mot9gwmy:f8ddce`, and the Coder was steered for the bounded `CODER_INTENT` step only.

## Implemented Slice 2026-05-05E

- `wp-receipt-append` now recognizes the common wrong-helper failure where a WP Validator tries `REVIEW_RESPONSE`/`wp-review-response` while the runtime is explicitly waiting on `WP_VALIDATOR_INTENT_CHECKPOINT` with `route_anchor_kind=VALIDATOR_RESPONSE`; it fails closed with the exact `just wp-validator-response ...` recovery path.
- Generated WP Validator steering prompts now separate early `CODER_INTENT` checkpoint clearance (`wp-validator-response`) from actual open review-item responses (`wp-review-response`), reducing model turns spent discovering the command split.
- Startup prompt command reference now includes `wp-validator-response` next to the review exchange helpers so governed role sessions see the direct checkpoint command at startup.
- Live recovery proof: after one rejected `wp-review-response`, the WP Validator published the correct `VALIDATOR_RESPONSE`, the lane advanced to `waiting_on=CODER_HANDOFF`, and focused receipt tests cover both the valid clearance path and the wrong-helper diagnostic.

## Implemented Slice 2026-05-05F

- Added `.GOV/roles_shared/docs/ORCHESTRATOR_MANAGED_WORKFLOW_PLAYBOOK.md` as the compact healthy-lane and stall-recovery map for orchestrator-managed ACP work.
- Codex `[CX-218K]` and the Orchestrator, Classic Orchestrator, WP Validator, Integration Validator, classic Validator, Memory Manager, and Activation Manager protocols now require the mechanical intervention discipline: classify 3-5 plausible causes before patching, steering, relaying, or declaring a blocked handoff.
- `justfile` repomem wrappers now pass explicit Orchestrator role/WP scope into mutation gates and context capture so concurrent Coder/WP Validator memory sessions cannot satisfy Orchestrator mutation authority.
- `repomem` now rejects unscoped legacy memory markers when a WP-scoped role session is required, and focused tests cover the regression.
- `wp-lane-health` now uses cursor-aware notification projection instead of raw notification rows for backlog reporting.
- Session health projection now marks active runs with missing command-output files as `DEGRADED` / `COMMAND_OUTPUT_MISSING` instead of projecting them as healthy.
- `buildSteeringPrompt` now drains queued turn-boundary nudges during normal steering prompts, not only startup prompts.
- `orchestrator-steer-next` now converts an `ESCALATED` route to a direct safe-boundary `SEND_PROMPT` when the target session is `READY` and no governed control request is already pending, preventing READY sessions from accumulating dead-letter nudges.
- Live recovery proof: the Coder nudge queue drained from depth 3 to 0, Coder moved from `READY` to `COMMAND_RUNNING`, and relay status moved from `ESCALATED` to `WATCH` while awaiting the next Coder receipt.
- Coder steering prompts now state that acking a `VALIDATOR_RESPONSE` notification does not satisfy a `CODER_HANDOFF` route; after ack, Coder must implement the cleared MT, commit it, and emit the required handoff/review receipt or report a concrete blocker.

## Implemented Slice 2026-05-06G

- `wp-relay-escalation` now treats target sessions with active governed runs as `WATCH` / `TARGET_SESSION_RUNNING_AWAITING_COMPLETION` instead of reclassifying the route as stale and recommending another steer while the broker is still executing the target role.
- `wp-lane-health` now suppresses receipt-age false positives while a Coder or WP Validator active run exists for the lane, and reports the receipt idleness as an informational active-run note.
- Codex and Coder/Orchestrator protocols now list `git restore` / `git checkout --` as destructive or state-hiding worktree rewrite commands that require explicit same-turn approval.
- The orchestrator-managed workflow playbook now includes a formatter/cleanup spillover pattern: classify 3-5 causes, compare dirty files with packet-cleared file targets, avoid broad formatter defaults during scoped MTs, and route typed blocker/repair instead of silently discarding spillover.
- Live recovery proof: during the PostgreSQL-primary WP, the Coder proof compile timed out under host load, broad `cargo fmt` had touched files beyond the cleared MT surfaces, and the lane then required a mechanical distinction between active-run waiting and cleanup-spillover risk. The hardened surfaces now make both cases visible.

## Implemented Slice 2026-05-06H

- `install-mt-hook` now resolves the effective post-commit hook path with `git -C <coder_worktree> rev-parse --git-path hooks/post-commit` instead of parsing the linked worktree `.git` file and writing an inert private-gitdir hook.
- `post-commit-mt-review-request` now logs `COMMIT_SUBJECT_NOT_MT` to the WP compile/relay log when a feat/WP commit subject does not match `feat: MT-NNN <description>`, instead of silently exiting.
- The same hook now runs its compile gate with `cargo check --manifest-path <worktree>/src/backend/handshake_core/Cargo.toml` and an external absolute `CARGO_TARGET_DIR`, instead of invoking bare `cargo check` at the worktree root where no `Cargo.toml` exists.
- Hook compile-gate timeouts are now classified as `COMPILE_TIMEOUT_REVIEW_SENT`: the review request continues with `HOOK_COMPILE_GATE=TIMEOUT_INCONCLUSIVE` so host-load timeouts do not silently dead-letter the MT relay while real compile failures still block review.
- Coder steering prompts now state that auto-relay is keyed to the exact `feat: MT-NNN <description>` subject shape; `fix: ...` or missing MT ids are ignored.
- `wp-review-exchange` now rejects malformed explicit `REVIEW_REQUEST` correlation ids such as stray summary words caused by shell quoting spillover, forcing the sender to rerun with safe quoting instead of recording corrupted route metadata.
- A focused regression test covers linked-worktree hook installation by asserting the installer writes to the Git-reported common hook path.
- The orchestrator-managed workflow playbook now includes a post-commit auto-relay failure pattern: classify 3-5 likely causes, verify effective hook path, verify `feat: MT-NNN` commit subject, send one typed manual review request when a valid commit already missed, and repair the hook before the next MT.
- Command reference plus Orchestrator/Coder protocols now document the hook contract and the manual fallback boundary.
- Live recovery proof: after Coder committed MT-001, the post-commit hook did not append a `REVIEW_REQUEST`; the first root cause was the hook installed under `.git/worktrees/wtc-plane-foundation-v1/hooks/post-commit` while Git reported the effective hook path as `.git/hooks/post-commit`. After reinstall, MT-002 still required manual review because the commit subject was `fix: fail closed without postgres storage URL`; the manual fallback then exposed summary/optional-field quoting spillover. MT-003 used the correct subject, then exposed the bare-root `cargo check` manifest miss. MT-004 exposed host-load compile-gate timeout as a route-silencing failure. Prompt, hook diagnostics, manifest-path compile gate, timeout classification, and review-helper validation now make those causes explicit.

## Implemented Slice 2026-05-06I

- `wp-communication-health-lib` now treats the Integration Validator's own open final `CODER_HANDOFF` as expected inbox state during `phase-check VERDICT`, while `phase-check HANDOFF` ignores that later final review item so committed WP Validator evidence can be recorded before final validation.
- `phase-check-lib` now passes role/session context into `VERDICT` communication health checks, including the recursive `CLOSEOUT` plan, so final-lane gates evaluate the role that is actually expected to act.
- `orchestrator-next` and `orchestrator-steer-next` now require durable committed handoff validation evidence before steering Integration Validator on a final `CODER_HANDOFF`, preventing closeout-first recovery loops that skip the committed range proof.
- Generated Integration Validator prompts now state that the first response to a final `CODER_HANDOFF` is `phase-check VERDICT`, followed by a correlation-preserving review response; `phase-check CLOSEOUT` is only terminal after the review/verdict response resolves the handoff.
- `just orchestrator-steer-next` and `just manual-relay-dispatch` now accept option-shaped second/third positionals as flags instead of treating them as repomem context/model values, so targeted commands such as `--target-role=... --target-session=... --direct` reach the relay script.
- The playbook and protocol references now document the final-handoff closeout inversion scenario with 3-5 cause triage: closeout-before-review, missing committed evidence, stage-unaware open-review filtering, ACK/status substituted for review receipt, and protocol prompt drift.
- Live recovery proof: the first Integration Validator attempt recorded `CLOSEOUT_PHASE_GATE_FAILED`; after patching, `phase-check HANDOFF ... WP_VALIDATOR --range ac9f8f...d7f3f760` and `phase-check VERDICT ... INTEGRATION_VALIDATOR integration_validator:...` both passed, and the corrected Integration Validator prompt was dispatched through the governed `session-send` lane.

## Implemented Slice 2026-05-06J

- Final Integration Validator `REVIEW_RESPONSE` receipts for final `CODER_HANDOFF` now preserve final-review routing instead of deriving a stale microtask fallback contract and notifying Coder for an unnecessary ack.
- `wp-communication-health-lib` now recognizes the final Integration Validator open+resolution pair as `COMM_OK` even when older runtime route anchors still point at Coder/microtask review traffic.
- `wp-receipt-append` suppresses Coder notifications for final Integration Validator direct-review resolutions while keeping Orchestrator checkpoint/routing truth.
- Live recovery proof: after the final PASS receipt at `2026-05-06T03:22:52.632Z`, a typed Orchestrator `REPAIR` receipt restored final-review route truth and `just wp-communication-health-check ... STATUS --verbose` returned `COMM_OK` with `integration_final_open=1` and `integration_final_resolution=1`.

## Implemented Slice 2026-05-06K

- `integration-validator-closeout-sync` can materialize a split, parser-compliant validation report from the final Integration Validator PASS receipt when packet `VALIDATION_REPORTS` is missing the required verdict block.
- The materialized report now uses top-level scalar fields, exact `CLAUSE_CLOSURE_MATRIX` row labels under `CLAUSES_REVIEWED`, concrete code/symbol evidence in counterfactual/current-main/negative-proof sections, and signed-scope-compatible residual risk wording.
- Closeout sync self-validation now throws into the rollback block instead of calling the process-exiting `fail()` path after packet/runtime writes, preventing partial packet edits when `validator-packet-complete` rejects generated truth.
- `validator-governance-lib` and `computed-policy-gate-lib` list parsing now stops at inline scalar labels, preventing later packet report instructions from being swallowed into `DATA_CONTRACT_GAPS` or other list fields.
- `packet-runtime-projection-lib` now allows `Done` + `MERGE_PENDING` to remain in the containment milestone after direct review is complete, and `orchestrator-next` treats `MERGE_PENDING` / `DONE_MERGE_PENDING` as terminal history rather than stale Activation-readiness-driven Coder delegation.
- Live recovery proof: `just phase-check CLOSEOUT WP-1-Postgres-Primary-Control-Plane-Foundation-v1 --sync-mode MERGE_PENDING --context "Integration Validator final PASS is resolved; record merge-pending closeout truth after orchestrator repaired communication routing" --sync-debug` passed, wrote `TERMINAL_CLOSEOUT_RECORD.json`, closed governed role sessions, and projected the WP as `MERGE_PENDING`.

## Implemented Slice 2026-05-06L

- Coder protocol now carries the CX-218K mechanical intervention discipline: before reporting or acting on a handoff stall, MT auto-relay miss, formatter spillover, proof delay, or helper/protocol mismatch, classify 3-5 plausible causes including runtime route drift, notification/cursor drift, session/ACP drift, documentation/protocol drift, clock/staleness drift, scope/worktree drift, and tool/proof failure.
- `protocol-alignment-check` now treats Codex, Orchestrator, Classic Orchestrator, Coder, WP Validator, Integration Validator, classic Validator, Memory Manager, Activation Manager, and the orchestrator-managed playbook as active alignment surfaces for CX-218K.
- The same check now fails if active surfaces lose the CX-218K marker, 3-5-cause language, documentation/protocol drift language, ACP/session ambiguity language, or the playbook reference where that role should use the playbook directly.
- This turns the Operator's retrospective steering rule into a routine `gov-check` guard instead of a prose-only convention for future parallel WP swarms.

## Implemented Slice 2026-05-06M

- `wp-lane-health` now resolves the current `resolveWorkPacketPath` object shape before reading packet metadata, so packet-declared runtime status files are not silently missed.
- Terminal packet/task-board projection now fences lane-health diagnostics: closed Coder/WP Validator rows, stale receipts, old notification cursors, hook/worktree auto-relay readiness, and relay-recovery blockers are hidden as terminal history instead of active stalls.
- The orchestrator-managed playbook's merge-pending terminal pattern now includes lane-health false stalls and packet-resolver drift as explicit likely causes.
- Live proof: `just wp-lane-health WP-1-Postgres-Primary-Control-Plane-Foundation-v1` now prints `Terminal WP: DONE_MERGE_PENDING`, reports one stale lane-history suppression, and exits with `HEALTH: OK` instead of warning that the closed WP Validator is not steerable or that the last Integration Validator receipt is stale.

## Implemented Slice 2026-05-06N

- `wp-lane-health` now calls the packet/task-board publication helper with the correct object-shaped input, and `protocol-alignment-check` guards that call shape so terminal publication truth cannot silently fall back to runtime-only history.
- `wp-relay-watchdog` and `wp-autonomous-monitor` now read packet status plus Task Board status through `readExecutionPublicationView`; terminal WPs return `TERMINAL_HISTORY_HIDDEN` or `terminal=YES publication=...` instead of waking closed roles.
- Role-local startup briefs for Classic Orchestrator, Activation Manager, WP Validator, Integration Validator, classic Validator, Memory Manager, and Orchestrator now carry CX-218K mechanical intervention cards. The startup brief schema and protocol-alignment check require those cards, cheapest deterministic helper language, and the no-manual-ordinary-relay rule.
- Active protocols now explicitly require the cheapest deterministic read/repair/typed helper before ordinary relay or blocker narration, reducing future swarm cost from repeated prose handoffs.
- The orchestrator-managed playbook now includes the main-containment path after Integration Validator PASS: safety pushes before merge, merge only from `../handshake_main`, ancestor proof for the approved target head, contained-main closeout, `gov-check`, and only then `origin/main` push.
- Live proof: `wp-relay-watchdog --observe-only` skipped the terminal PostgreSQL-primary WP with `TERMINAL_HISTORY_HIDDEN`; `wp-autonomous-monitor --once` logged `terminal=YES publication=DONE_MERGE_PENDING`; `phase-check CLOSEOUT --sync-mode CONTAINED_IN_MAIN --merged-main-sha 00fda21a394278ca1fa105df972ffac8b9f4d11e` passed and projected the packet as `Validated (PASS)` / `CONTAINED_IN_MAIN`.

## Verification Plan

- Run focused ACP/session tests after implementation patches.
- Run `just session-registry-status WP-1-Postgres-Primary-Control-Plane-Foundation-v1` during the active workflow.
- Run `just gov-check` before any governance commit that changes live governance surfaces.
