# COMMAND_SURFACE_REFERENCE

**Status:** Draft  
**Intent:** canonical operator-facing `just` command surface for current governance workflow  
**Authority note:** the live executable surface is the root `justfile`; if this document disagrees with `just --list`, treat this document as stale and fix it
**Startup prompt note:** this document is an operator cheat sheet only. Governed role startup prompts must be derived from `.GOV/roles_shared/scripts/session/session-control-lib.mjs::buildStartupPrompt`, not hand-maintained here.

## Purpose

This file groups the live `just` surface by workflow purpose so roles do not have to infer command meaning from protocol prose alone.

For the exhaustive inventory, run:

```bash
just --list
```

## Reading key

- `read-only`: does not intentionally mutate packet/runtime state
- `runtime-write`: mutates external runtime ledgers, broker state, session state, or packet communication state
- `governance-write`: mutates governed files under `/.GOV/`
- `product-scan`: reads product code and is expected to run from the appropriate product worktree context

## Operator-facing scope split

Use this split in chat every time scope, remediation, or next steps are discussed:

- `Handshake (Product)`
  - product code, product tests, Master Spec requirements, product WPs
  - includes product-governance contract work such as governed actions, queue law, workflow-state semantics, or runtime contract enforcement when the diff touches `src/`, `app/`, `tests/`, or the Master Spec
- `Repo Governance`
  - `/.GOV/**`, ACP/session/runtime ledgers, role protocols, governance task-board/changelog/audits, root control-file maintenance
  - this lane does not use a WP when the planned diff stays governance-only
- If only one lane applies, still name both lanes and state `NONE` for the other lane.

## Governance health and read-only status

These are safe starting points for orientation and health checks.

- `just gov-check`
  - `read-only`
  - shared governance health; expected before new governed execution and before final closure
- `just docs-check`
  - `read-only`
  - presence check for required governance docs
- `just canonise-gov`
  - `read-only`
  - inspects the canonisation file set for governance drift and prints the mandatory review checklist; after running it, inspect every listed file and update applicable drift before closeout
- `just artifact-hygiene-check`
  - `read-only`
  - validates external artifact placement; repo-local `target/` directories and blocking non-canonical `Handshake Artifacts` residue fail closed
  - retention policy authority: `.GOV/roles_shared/docs/ARTIFACT_RETENTION_POLICY.md`
- `just session-registry-status [WP-{ID}]`
  - `read-only`
  - inspect governed session state; when a WP filter is supplied, this now also prints the governed WP token-usage rollup by role, derived stalled-relay status, the active terminal batch id, and owned-terminal metadata/reclaim status
- `just active-lane-brief <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR> WP-{ID} [--json]`
  - `read-only`
  - print the compact authority/context digest for one governed role lane, including runtime route, notifications, relay health, declared microtask plan (`active` / `next`), and next commands
- `just manual-relay-next WP-{ID} [--debug]`
  - `read-only`
  - operator-facing next-step helper for `WORKFLOW_LANE=MANUAL_RELAY`; prints the runtime-projected next actor, target session, a structured relay envelope (`RELAY_ENVELOPE`, `ROLE_TO_ROLE_MESSAGE`, `OPERATOR_EXPLAINER`), and exact governed follow-up commands without auto-steering
- `just manual-relay-dispatch WP-{ID} [PRIMARY|FALLBACK] [--debug]`
  - `runtime-write`
  - operator-invoked broker for `WORKFLOW_LANE=MANUAL_RELAY`; starts the projected target session when needed, immediately delivers the active role-to-role payload, and injects typed relay context (`MANUAL_RELAY_CONTEXT`, `DIRECT_ROLE_MESSAGE`) into the target prompt instead of a generic resume-only steer
- `just wp-token-usage WP-{ID}`
  - `read-only`
  - print the governed per-WP token ledger aggregated from settled ACP session outputs
- `just wp-timeline WP-{ID} [--json]`
  - `read-only`
  - print one merged WP timeline plus structured span rows for control commands, token-command windows, review exchanges, and microtask execution windows, together with stage counts, token totals, and budget health
  - the summary now includes `relay_policy`: measured relay prompt burden for the current WP plus the default lane recommendation (`MANUAL_RELAY` unless autonomous steering is explicitly worth the extra prompt tax)
- `just wp-token-usage-settle WP-{ID} [REASON] [SETTLED_BY]`
  - `writes-runtime`
  - settle a historical WP token ledger to raw ACP session outputs after the lane is terminal so compact views stop surfacing old drift as live noise
- `just handshake-acp-broker-status`
  - `read-only`
  - inspect ACP broker liveness/state
- `just operator-viewport`
  - `read-only`
  - canonical operator viewport across sessions, receipts, control results, and packet/runtime activity
- `just operator-monitor`
  - `read-only`
  - compatibility alias for `just operator-viewport`
- `just orchestrator-next [WP-{ID}] [--debug]`
- `just coder-next [WP-{ID}]`
- `just validator-next [WP-{ID}] [--debug]`
  - `read-only`
  - role-specific resume helpers after startup/reset/compaction
  - for `WORKFLOW_LANE=ORCHESTRATOR_MANAGED`, post-signature routine Operator interruptions are invalid; `just orchestrator-next` should print `OPERATOR_ACTION: NONE` unless a machine-visible `BLOCKER_CLASS` is present
- `just orchestrator-steer-next WP-{ID} [PRIMARY|FALLBACK]`
  - `runtime-write`
  - launch or steer the next expected governed actor directly from runtime/receipt projection without a manually written relay prompt
  - if the target session is not running yet, this helper now starts it and immediately sends the typed route payload in the same invocation
  - the governed prompt carries typed route context (`GOVERNED_ROUTE_CONTEXT`, `DIRECT_ROLE_MESSAGE`) derived from receipt/notification truth instead of generic resume prose
  - when stalled-relay escalation is active, this is the canonical continue/repair command instead of silent waiting
- `just manual-relay-next WP-{ID} [--debug]`
  - `read-only`
  - for `WORKFLOW_LANE=MANUAL_RELAY`, inspect runtime next-actor truth without dispatching any prompt
- `just manual-relay-dispatch WP-{ID} [PRIMARY|FALLBACK] [--debug]`
  - `runtime-write`
  - for `WORKFLOW_LANE=MANUAL_RELAY`, let the operator explicitly broker one governed start/send action against the currently projected next actor

## Governance Memory System (RGF-115 through RGF-143)

- `just memory-add <episodic|semantic|procedural> <topic> "<summary>" [--wp WP-{ID}] [--scope "files"] [--content "<full>"] [--source "<artifact>"] [--role "<role>"]`
  - `runtime-write`
  - record a new memory entry with FTS5 indexing; provider-agnostic, any model can write
- `just memory-search "<query>" [--type <type>] [--wp WP-{ID}] [--limit N]`
  - `read-only`
  - FTS5 keyword search over all memory layers; returns matching index entries with content preview
- `just memory-prime <WP-{ID}> [--files "file1,file2"] [--desc "<description>"] [--budget N]`
  - `read-only`
  - returns MT-scoped relevant memories within a token budget; designed for injection into session startup
- `just memory-stats`
  - `read-only`
  - database size, entry counts by type, schema version, last compaction, oldest active entry
- `just memory-decay [--rate 0.1] [--threshold 0.05]`
  - `runtime-write`
  - apply importance decay to all active memories; prune entries below threshold; log run
- `just memory-migrate-failure-memory`
  - `runtime-write`
  - one-time migration of legacy FAILURE_MEMORY.json entries into the governance memory SQLite store (migration complete; JSON archived as `.migrated`)
- `just memory-export [--all]`
  - `read-only`
  - dump active memories to JSONL on stdout for git-trackable archival; `--all` includes consolidated entries
- `just memory-import <file.jsonl>`
  - `runtime-write`
  - restore memories from a JSONL export; deduplicates by topic+wp+type
- `just memory-extract [WP-{ID}|--all]`
  - `runtime-write`
  - extract episodic and procedural memories from WP RECEIPTS.jsonl; `--all` processes every WP with communications
- `just memory-extract-smoketests [<file.md>]`
  - `runtime-write`
  - extract findings (SMOKE-FIND-*) and positive controls (SMOKE-CONTROL-*) from smoketest reviews into semantic/procedural memory
- `just memory-compact [--older-than 30d] [--dry-run]`
  - `runtime-write`
  - full maintenance cycle: dedup, episodic→semantic consolidation, importance decay, orphan cleanup; `--dry-run` for preview
- `just memory-embed [--batch N]`
  - `runtime-write`
  - generate nomic-embed-text embeddings via local Ollama for unembedded memories; default batch=20; requires Ollama running on localhost:11434
- `just memory-hybrid-search "<query>" [--type <type>] [--wp WP-{ID}] [--limit N]`
  - `read-only`
  - combine FTS5 keyword + vector cosine similarity via Reciprocal Rank Fusion; requires embeddings (run `just memory-embed` first)
- `just memory-capture <procedural|semantic|episodic> "<insight>" [--wp WP-{ID}] [--scope "files"] [--role "<role>"]`
  - `runtime-write`
  - mid-session memory capture for roles; importance 0.7; callable by coders/validators during active work to record fix patterns, systemic issues, or session insights [RGF-127]
- `just memory-flag <memory-id> "<reason>"`
  - `runtime-write`
  - suppress a bad/misleading memory: sets importance to 0.1, records flag reason in metadata; flagged memories are deprioritized in injection until reviewed
- `just memory-intent-snapshot "<what you are about to do>" [--wp WP-{ID}] [--role ROLE] [--reason "<why>"] [--expected "<outcome>"] [--scope "files"]`
  - `runtime-write`
  - judgment-based context+intent capture before complex reasoning tasks; importance 0.9; roles call this when protocol requires it (refinement, deep review, research, refactor batches); no mechanical trigger — the model decides when to use it; 120s dedup window
- `just memory-debug-snapshot [WP-{ID}|<SNAPSHOT_TYPE>] [--wp WP-{ID}] [--type <type>] [--limit N]`
  - `read-only`
  - inspect pre-task and intent snapshots; shows snapshot type, WP, timestamp, and structured context payload; use `INTENT` as type filter to see only intent snapshots [RGF-144]
- `just memory-patterns [--min-wps N] [--min-access N]`
  - `read-only`
  - cross-WP pattern synthesis: clusters similar memories, finds recurring smoketest failures, repeated REPAIR transitions, and high-access systemic patterns; outputs governance improvement candidates report [RGF-129]
- `just memory-refresh [--force-compact]`
  - `runtime-write`
  - extract new memories from receipts + smoketests, then run compaction if stale (>24h with dual-gate); called automatically at every role startup + gov-check; `--force-compact` bypasses staleness check

### Conversation memory (`just repomem`)

- `just repomem open "<what this session is about>" [--role ROLE] [--wp WP-ID]`
  - `runtime-write`
  - **MANDATORY** at session start. Creates session marker, writes SESSION_OPEN checkpoint. All mutation commands are blocked (via `repomem-gate`) until this runs. Content >=80 chars enforced. Shows prior session context on success.
- `just repomem pre "<about to do X because Y>" [--wp WP-ID] [--trigger "just cmd"]`
  - `runtime-write`
  - pre-task checkpoint before an action; content >=40 chars; requires active session
- `just repomem insight "<key realization or operator decision>" [--wp WP-ID] [--files "a,b"] [--decisions "what was decided"]`
  - `runtime-write`
  - **MANDATORY** after operator decisions/corrections and after non-obvious discoveries. Content >=80 chars. This is the primary mechanism for capturing institutional knowledge across sessions.
- `just repomem research-close "<what was found>" [--wp WP-ID] [--files "a,b"] [--decisions "conclusions"]`
  - `runtime-write`
  - research conclusion checkpoint; content >=80 chars
- `just repomem close "<session summary>" --decisions "<key decisions made>"`
  - `runtime-write`
  - **MANDATORY** at session end. Content >=80 chars, `--decisions` required. Shows session checkpoint summary. Clears session marker.
- `just repomem context "<why this action>" --trigger "<just cmd>"` 
  - `runtime-write`
  - piggybacked context for mutation commands; content >=40 chars; auto-called by `task-board-set`, `create-task-packet`, `orchestrator-steer-next`, `manual-relay-dispatch`, `phase-check CLOSEOUT --sync-mode ...`, `begin-refinement`, `begin-research`, `wp-traceability-set`
- `just repomem log [--session last|current] [--week] [--month] [--search "<query>"] [--wp WP-ID] [--limit N]`
  - `read-only`
  - view conversation history; `--session last` shows full prior session; `--week`/`--month` for time range; `--search` for FTS keyword search
- `just repomem gate`
  - `read-only`
  - check if SESSION_OPEN exists; exits 1 if not; used by mutation commands as a blocking gate

### Mutation commands requiring `context` parameter

The following commands now require a `context` string (>=40 chars) that is captured as a conversation checkpoint before the command runs:

- `just task-board-set WP-{ID} <STATUS> "<context>" ["reason"]`
- `just create-task-packet WP-{ID} "<context>"`
- `just orchestrator-steer-next WP-{ID} "<context>" [MODEL] [FLAGS]`
- `just manual-relay-dispatch WP-{ID} "<context>" [MODEL] [FLAGS]`
- `just phase-check CLOSEOUT WP-{ID} [ROLE] [session] --sync-mode <MERGE_PENDING|CONTAINED_IN_MAIN|FAIL|OUTDATED_ONLY|ABANDONED> --context "<context>" [--merged-main-sha <SHA>] [--sync-debug]`
- `just wp-traceability-set BASE_WP ACTIVE_WP "<context>"`
- `just begin-refinement WP-{ID} "<intent>"` (intent serves as context, >=40 chars)
- `just begin-research "<intent>"` (intent serves as context, >=40 chars)

### Deprecated (redirected to governance memory DB)

- `just failure-memory-record` → use `just memory-capture procedural "<fix>" --scope "<file>" --wp WP-{ID}` instead
- `just failure-memory-query` → use `just memory-search "<query>"` instead

These legacy commands still work (they redirect to the governance memory DB) but will be removed in a future version. The legacy `FAILURE_MEMORY.json` has been migrated and archived.

## Governance flush (full sync cycle)

- `just gov-flush`
  - `governance-write` + `runtime-write`
  - deterministic governance flush pipeline: commit dirty .GOV/ files + push gov_kernel, sync gov to main, push main, reseed wt-ilja, push user_ilja, artifact cleanup (dry-run then actual, no force delete), NAS backup (only if cleanup succeeded)
  - reports all committed files back in the output
  - artifact cleanup failure is reported but does not undo the commits, pushes, and syncs that preceded it
  - run after a governance session to propagate all changes and secure them on GitHub + NAS

## Orchestrator workflow helpers

- `just begin-refinement WP-{ID} "<intent>"`
  - `runtime-write`
  - captures an intent snapshot and opens the refinement gate for a WP; use before starting scope analysis and feature discovery
- `just begin-research "<intent>" [--wp WP-{ID}] [--role ROLE]`
  - `runtime-write`
  - captures an intent snapshot and opens a research pass; use before governance research or cross-WP analysis
- `just generate-refinement-rubric [args]`
  - `read-only`
  - generate a structured refinement rubric for WP scope evaluation
- `just spec-debt-sync WP-{ID}`
  - `governance-write`
  - synchronise spec debt tracking for a WP
- `just wp-closeout-format WP-{ID} <MERGED_MAIN_COMMIT>`
  - `read-only`
  - format the closeout block for a validated WP after merge
- `just wp-traceability-set <BASE_WP_ID> <ACTIVE_PACKET_WP_ID>`
  - `governance-write`
  - set traceability mapping between a base WP and its active packet (supersession, versioning)
- `just wp-lane-health WP-{ID}`
  - `read-only`
  - inspect lane health for a WP: session liveness, relay state, stall detection
- `just session-stall-scan <ROLE> WP-{ID}`
  - `read-only`
  - scan a governed session lane for stall conditions

## Microtask management

- `just mt-board WP-{ID}`
  - `read-only`
  - view the microtask board for a WP: status, claims, completion
- `just mt-claim WP-{ID} <SESSION_KEY>`
  - `runtime-write`
  - claim the next available microtask for a governed session
- `just mt-complete WP-{ID} <MT_ID>`
  - `runtime-write`
  - mark a microtask as complete
- `just mt-populate WP-{ID}`
  - `governance-write`
  - populate microtask files from packet scope into the MT board
- `just install-mt-hook WP-{ID}`
  - `runtime-write`
  - install the microtask commit hook for a WP branch
- `just install-validator-guard WP-{ID}`
  - `runtime-write`
  - install the validator guard hook for a WP branch

## Operator admin

- `just operator-viewport-admin [args]`
  - `runtime-write`
  - admin mode for operator viewport: manage sessions, force-settle, clear state
- `just operator-admin [args]`
  - `runtime-write`
  - alias for `just operator-viewport-admin`
- `just handshake-acp-broker-stop`
  - `runtime-write`
  - stop the ACP broker process
- `just launch-memory-manager [FLAGS]`
  - `runtime-write`
  - launch the governance memory manager role session

## Internal checks and sub-recipes

These are called by higher-level recipes (`gov-check`, role startup) and are not normally invoked directly. Listed for command surface completeness.

- `just validator-spec-regression`
  - `read-only`
  - verify spec file presence and required anchors
- `just cor701-sha <FILE>`
  - `read-only`
  - compute and verify SHA for a governed file
- `just spec-eof-appendices-check`
  - `read-only`
  - verify spec EOF structure and appendices
- `just session-control-runtime-check`
  - `read-only`
  - verify session control runtime files exist
- `just protocol-ack <CODEX> <AGENTS> <SHARED> <PROTOCOL>`
  - `read-only`
  - emit protocol acknowledgment block for startup
- `just orchestrator-startup-truth-check`
  - `read-only`
  - verify orchestrator startup truth: active WPs, task board consistency

## Minimal Live Read Set (Token Discipline)

After startup and assignment, roles should usually be able to operate from a small live read set:

- startup output
- the assigned packet
- the active WP thread / notifications
- this command-surface reference when command choice is unclear
- `just active-lane-brief <ROLE> WP-{ID}` when packet/runtime/session truth feels fragmented

Repeated full rereads of large governance protocols, repeated `just --list`-style command rediscovery, and repeated path/source-of-truth checks after context is already stable should be treated as ambiguity smells, not as normal diligence.

For orchestrator-managed lanes after signature/prepare:

- routine Operator asks such as "proceed", checkpoint approval, or generic approval relapse are invalid
- real escalations must name one `BLOCKER_CLASS`: `POLICY_CONFLICT`, `AUTHORITY_OVERRIDE_REQUIRED`, `OPERATOR_ARTIFACT_REQUIRED`, or `ENVIRONMENT_FAILURE`
- legacy pre-launch repair may still surface `LEGACY_SIGNATURE_TUPLE_REPAIR` from `just orchestrator-next`
- if the Operator explicitly authorizes bounded continuation after `TOKEN_BUDGET_EXCEEDED`, record it in the packet `## WAIVERS GRANTED` section as an active `GOVERNANCE` waiver that explicitly mentions `TOKEN_BUDGET_EXCEEDED` or `POLICY_CONFLICT`; `just orchestrator-next` may then continue while still surfacing the waiver in resume output

If a role keeps needing those rereads:

- prefer tightening the startup prompt, packet, command surface, or helper command
- record the churn in the next smoketest review under `Silent Failures, Command Surface Misuse, and Ambiguity Scan`

## Startup and preflight

- `just orchestrator-startup`
- `just coder-startup`
- `just validator-startup`
  - `read-only`
  - protocol ack + backup context + role preflight
  - governed startup prompts are derived from `session-control-lib.mjs` and now explicitly include `AGENTS.md + .GOV/codex/Handshake_Codex_v1.4.md + role protocol + startup output + packet`
- `just orchestrator-preflight`
- `just coder-preflight`
- `just validator-preflight`
  - `read-only`
  - compact gate bundles for startup
- `just hard-gate-wt-001`
  - `read-only`
  - print worktree/branch verification block for chat

## Governance maintenance (no WP)

Use this flow only for repo-governance maintenance that stays out of product code and the Master Spec.

- Working records:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `.GOV/Audits/**` with stable `AUDIT_ID` and, for smoketest reviews, `SMOKETEST_REVIEW_ID`
- Working templates:
  - `.GOV/templates/REPO_GOVERNANCE_TASK_ITEM_TEMPLATE.md`
  - `.GOV/templates/REPO_GOVERNANCE_CHANGELOG_TEMPLATE.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
- Shared workflow note:
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
- Commands:
- `just gov-check`
  - `read-only`
  - mandatory verification before claiming governance-maintenance completion
- `just build-order-sync`
  - `governance-write`
  - required only when governance changes affect `TASK_BOARD.md` or `WP_TRACEABILITY_REGISTRY.md`
- `just artifact-cleanup [--dry-run]`
  - `runtime-write`
  - removes only reclaimable stale external artifact folders and repo-local `target/` residue; closeout now runs this mechanically before containment sync
  - writes a retention manifest under `../Handshake Artifacts/handshake-tool/artifact-retention/`
- `just sync-gov-to-main`
  - `governance-write`
  - mirrors kernel `/.GOV/` into `handshake_main` and auto-commits on local `main`
  - requires committed kernel governance truth; if `wt-gov-kernel/.GOV` is dirty, commit `gov_kernel` first instead of syncing an uncommitted snapshot
- `just ensure-wp-communications WP-{ID}`
  - `runtime-write`
  - rebuild or repair the packet-declared communication artifacts under external runtime; this is the sanctioned repair helper when communications bootstrap drift is suspected

## Packet activation and governance state writes

These mutate packet, board, traceability, or related governed surfaces.

- `just record-refinement WP-{ID}`
- Refinement visibility rule:
  - the Operator must see the refinement in assistant-authored chat text before any signature request
  - shell/tool output does not count as "shown in chat"
  - if the refinement is too large for one message, paste it verbatim across multiple consecutive chat messages before requesting approval
- `just record-signature WP-{ID} <signature> <workflow_lane> <execution_lane>`
- `just record-role-model-profiles WP-{ID} [ORCHESTRATOR_MODEL_PROFILE] [CODER_MODEL_PROFILE] [WP_VALIDATOR_MODEL_PROFILE] [INTEGRATION_VALIDATOR_MODEL_PROFILE]`
- `just record-prepare WP-{ID} [workflow_lane] [execution_lane] [branch] [worktree_dir]`
  - `governance-write`
  - orchestrator-owned workflow state writes
  - `record-role-model-profiles` is the explicit per-role model/CLI policy gate for new packet families; omit args to record deliberate defaults (`OPENAI_GPT_5_4_XHIGH` for all roles)
  - `CLAUDE_CODE_OPUS_4_6_THINKING_MAX` is now a valid declared packet profile, but governed launch/control remains fail-closed until provider-specific runtime support is implemented
- `just create-task-packet WP-{ID}`
  - `governance-write`
  - packet creation from the template
  - for `PACKET_FORMAT_VERSION >= 2026-04-06`, packet creation is blocked until `just record-role-model-profiles` has recorded the authoritative per-role bundle
  - for `PACKET_FORMAT_VERSION >= 2026-04-01`, treat packet creation as law activation, not mere scaffolding: inspect `DATA_CONTRACT_PROFILE`, `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`, and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4` before delegation
  - on that packet family, coder handoff must include anti-vibe + signed-scope-debt self-audit; validator PASS requires both lists to be exactly `- NONE`
  - for `PACKET_FORMAT_VERSION >= 2026-04-05` and `RISK_TIER=MEDIUM|HIGH`, validator closeout is dual-track: PASS requires both `MECHANICAL_TRACK_VERDICT=PASS` and `SPEC_RETENTION_TRACK_VERDICT=PASS`
  - if `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, ensure `DATA_CONTRACT_MONITORING` is credible at packet create time; validator closeout later requires concrete `DATA_CONTRACT_PROOF` plus `DATA_CONTRACT_GAPS`
- `just orchestrator-prepare-and-packet WP-{ID}`
  - `governance-write`
  - transactional prepare + packet creation + sync flow
- `just task-board-set WP-{ID} <STATUS> ["reason"]`
  - use `DONE_MERGE_PENDING` after validator PASS append but before merge-to-main containment
  - use `DONE_VALIDATED` only after the approved closure commit is actually contained in local `main`
- `just build-order-sync`
  - `governance-write`
  - projection updates
- `just post-run-audit-skeleton WP-{ID} [output]`
  - `read-only`
  - generate audit skeleton from current authoritative artifacts

## Session launch and steering (Orchestrator-only)

These mutate governed runtime state and should not be run from inside Coder or Validator sessions.
For Orchestrator-managed WPs, this ACP/CLI session surface is the required normal delegation path.
For an active orchestrator-managed WP, helper agents/subagents are not allowed to perform coder, validator, or in-lane review/steering duties. Governed ACP sessions are the only legal execution lanes for `CODER`, `WP_VALIDATOR`, and `INTEGRATION_VALIDATOR`.
If the Operator explicitly authorizes separate governance-only helper work outside the active lane, keep it isolated and do not let it write product code unless the packet records `SUB_AGENT_DELEGATION: ALLOWED` plus exact `OPERATOR_APPROVAL_EVIDENCE`.

- `just launch-coder-session WP-{ID} [AUTO|PRINT|CURRENT|SYSTEM_TERMINAL|VSCODE_PLUGIN] [PRIMARY|FALLBACK]`
- `just launch-wp-validator-session WP-{ID} ...`
- `just launch-integration-validator-session WP-{ID} ...`
  - `runtime-write`
  - launch/bootstrap lane
  - launch selection now resolves through the packet-declared role-model profile bundle, not only implicit GPT defaults
  - on the ordinary orchestrator-managed path, supported launch hosts now auto-issue the first governed `START_SESSION` so launch does not stop at a launch-only false green
  - governed launch/control must preserve kernel governance authority with `HANDSHAKE_GOV_ROOT=<wt-gov-kernel>/.GOV`; `handshake_main/.GOV` is not valid live governance for orchestrator-managed integration validation
- `just start-coder-session WP-{ID} [PRIMARY|FALLBACK]`
- `just start-wp-validator-session WP-{ID} ...`
- `just start-integration-validator-session WP-{ID} ...`
  - `runtime-write`
  - explicit governed ACP start / recovery helper when a launch host could not complete the first start automatically
- `just steer-coder-session WP-{ID} "<prompt>" [PRIMARY|FALLBACK]`
- `just steer-wp-validator-session WP-{ID} ...`
- `just steer-integration-validator-session WP-{ID} ...`
  - `runtime-write`
  - governed ACP resume/send
- `just cancel-coder-session WP-{ID}`
- `just cancel-wp-validator-session WP-{ID}`
- `just cancel-integration-validator-session WP-{ID}`
  - `runtime-write`
  - cancel the current governed command for that lane
- `just close-coder-session WP-{ID}`
- `just close-wp-validator-session WP-{ID}`
- `just close-integration-validator-session WP-{ID}`
  - `runtime-write`
  - retire steerable thread registration for that lane and attempt deterministic reclaim of any governed system-terminal window owned by that exact session
- Generic wrappers:
- `just session-start <ROLE> WP-{ID} [PRIMARY|FALLBACK]`
- `just session-send <ROLE> WP-{ID} "<prompt>" [PRIMARY|FALLBACK]`
- `just session-cancel <ROLE> WP-{ID}`
- `just session-close <ROLE> WP-{ID}`
    - these governed helpers now attempt deterministic self-settlement for their own request ids when a broker dispatch or wait path returns without a terminal result row
- `just session-reclaim-terminals WP-{ID} [CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR] [CURRENT_BATCH|ALL_BATCHES|<BATCH_ID>]`
  - `runtime-write`
  - manual repair helper that reclaims only registry-owned governed system-terminal windows for the selected WP/session scope; it defaults to `CURRENT_BATCH` so older batch windows are left alone unless `ALL_BATCHES` or an exact `BATCH_ID` is requested

## Packet communication surface

These operate on the packet-declared `WP_COMMUNICATION_DIR` under external runtime.

- `just wp-thread-append ...`
  - `runtime-write`
  - soft coordination only
- `just wp-heartbeat ...`
  - `runtime-write`
  - liveness and actor-local phase projection only
  - `next_actor`, `waiting_on`, and related route fields are assertion-only against current runtime truth; heartbeat must not be used to change workflow routing or validator-readiness semantics
- `just wp-receipt-append ...`
  - `runtime-write`
  - low-level deterministic receipt append
- `just wp-invalidity-flag WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> <INVALIDITY_CODE> ...`
  - `runtime-write`
  - records a machine-visible `WORKFLOW_INVALIDITY` receipt and routes attention back to the Orchestrator
- `just wp-operator-rule-restatement WP-{ID} <ACTOR_ROLE> <ACTOR_SESSION> "<summary>" ...`
  - `runtime-write`
  - specialized invalidity helper for the case where the Operator had to restate a core orchestrator-managed rule; projects `LANE_RESET_REQUIRED` instead of generic invalidity drift
- `just wp-validator-kickoff ...`
- `just wp-coder-intent ...`
- `just wp-coder-handoff ...`
- `just wp-validator-review ...`
- `just wp-validator-response ...`
- `just wp-review-response ...`
- `just wp-review-exchange <RECEIPT_KIND> ...`
- `just wp-spec-gap ...`
- `just wp-spec-confirmation ...`
  - `runtime-write`
  - structured direct-review / review-resolution helpers
  - validator-owned bootstrap/skeleton gate: on governed lanes, after `wp-coder-intent` the lane now requires one explicit WP-validator checkpoint before implementation hardens or full `wp-coder-handoff` is legal; use `wp-validator-response` to clear the early plan or `wp-spec-gap` / `VALIDATOR_QUERY` to keep the lane in early review
  - optional final `microtask_json` argument may carry a compact steering contract with `scope_ref`, `file_targets`, `proof_commands`, `risk_focus`, `expected_receipt_kind`, `review_mode`, `phase_gate`, and `review_outcome`
  - when the resolved Work Packet folder contains `MT-*.md` files (current physical storage: `.GOV/task_packets/WP-{ID}/MT-*.md`) on an orchestrator-managed lane, governed coder `wp-coder-intent` and overlap `REVIEW_REQUEST` receipts now fail closed unless `microtask_json.scope_ref` resolves to one declared MT (`MT-001` or `CLAUSE_CLOSURE_MATRIX/CX-...`), `file_targets` are concrete, and those targets stay inside that MT's `CODE_SURFACES`
  - use `phase_gate=BOOTSTRAP` or `phase_gate=SKELETON` when the receipt is part of that mandatory early validator gate
  - rolling microtask overlap: use `wp-review-exchange REVIEW_REQUEST ...` with `review_mode=OVERLAP` for completed narrow slices while the coder advances the next declared microtask; the unresolved overlap queue is bounded to 2 and full `wp-coder-handoff` is blocked until those overlap review items are drained
- `just phase-check <STARTUP|HANDOFF|VERDICT|CLOSEOUT> WP-{ID} [ROLE] [session]`
  - `read-only` by default; `CLOSEOUT` becomes `governance-write` when `--sync-mode ... --context ...` is supplied
  - canonical phase-boundary gate entrypoint
  - `STARTUP`: is the canonical startup/bootstrapping gate; for `CODER` it owns the packet/startup proof that used to live behind `pre-work`, and for validator roles it proves the startup communication mesh before productive work starts
  - `HANDOFF`: proves coder closure or validator handoff readiness from one phase artifact, depending on role
  - `VERDICT`: proves the final review communication boundary from one phase artifact
  - `CLOSEOUT`: runs the verdict bundle, emits the integration-validator context brief, proves closeout readiness, and refreshes memory-manager maintenance; with `--sync-mode ... --context ...` it also performs the governed packet/runtime/TASK_BOARD closeout sync inside the same phase artifact
- `just wp-communication-health-check WP-{ID} [STATUS|KICKOFF|HANDOFF|VERDICT]`
  - `read-only`
  - low-level communication proof and route health; phase-level role guidance should usually prefer the canonical `phase-check` entrypoint above
- `just check-notifications WP-{ID} <ROLE> [session] [--history]`
  - `read-only`
  - inspect unread notifications; the default view projects unread history down to the active blocking route for that role/session, so pass the governed actor session to avoid same-role cross-session leakage and use `--history` only when you need suppressed terminal or superseded residue
- `just ack-notifications WP-{ID} <ROLE> <session>`
  - `runtime-write`
  - acknowledge notifications for one governed session only

## Coder execution surface

These are typically run from the WP-assigned worktree.

- `just phase-check STARTUP WP-{ID} CODER [session] [--verbose]`
  - `read-only`
  - blocking startup gate for coder work
  - default output is compact-by-default and writes the full nested gate output to a governed runtime artifact path
- `just phase-check HANDOFF WP-{ID} CODER [--range ... | --rev ... | --verbose]`
  - `read-only`
  - deterministic closure gate against the validated diff window
- `just coder-skeleton-checkpoint WP-{ID}`
- `just skeleton-approved WP-{ID}`
  - `governance-write`
  - docs-only phase-boundary helpers for `MANUAL_RELAY` only
  - forbidden on `ORCHESTRATOR_MANAGED`; those invocations now fail and record `WORKFLOW_INVALIDITY`
- `just product-scan`
- `just validator-dal-audit`
- `just validator-git-hygiene`
  - `product-scan`
  - coder hygiene surface before handoff
- work-packet path note:
  - the logical Work Packet resolver name is `work_packets`, but the current physical storage root remains `.GOV/task_packets/` during compatibility migration. Scripts should resolve packet paths through `runtime-paths.mjs`, not by hard-coding folder names.
  - reserved archive roots now exist under `.GOV/task_packets/_archive/` for `superseded/` and `validated_closed/`; do not move packets there by hand, but the resolver already understands those future archive targets
- `just cargo-clean`
  - `product-scan`
  - workspace cleanup targeting `handshake_core`

## Validator execution surface

These are usually run from the WP worktree for WP-validator work or from `handshake_main` for integration-validator/final validation work.

- `just phase-check <STARTUP|HANDOFF|VERDICT|CLOSEOUT> WP-{ID} [ROLE] [session]`
  - `read-only` by default; `CLOSEOUT --sync-mode ... --context ...` is the preferred canonical governed closeout writer
  - canonical validator-facing phase-boundary gate
  - `HANDOFF`, `VERDICT`, and `CLOSEOUT` are the preferred role-facing entrypoints and the only live phase-gate commands
- `just integration-validator-context-brief WP-{ID} [--json]`
- `just wp-declared-topology-check WP-{ID}`
- `just validator-policy-gate WP-{ID}`
    - `read-only`
    - support/debug surfaces adjacent to the canonical `phase-check` boundary
    - `integration-validator-context-brief` is the canonical final-lane authority/path/source-of-truth bundle for orchestrator-managed Integration Validator review; use it instead of rereading large protocols or rediscovering final-lane paths/commands
    - default text output is compact-by-default and points at the authoritative packet/gate artifacts; use `--json` for the full machine-readable brief
    - `phase-check CLOSEOUT ... --sync-mode ... --context ...` is the preferred governed writer because it keeps closeout proof, truth sync, and final memory refresh inside one phase-owned artifact
    - for orchestrator-managed final review, live governance authority still comes from `wt-gov-kernel/.GOV`; `handshake_main/.GOV` is only the synced main-branch mirror and must not be treated as the live authority surface
    - candidate-target validation remains exact to the signed artifact; contained local-main closure may include conflict-resolved harmonization only when the contained commit stays inside the signed file surface and the governed closeout proof still passes
    - `wp-declared-topology-check` surfaces packet-declared vs actual linked-worktree truth for one WP and fails on undeclared auxiliary worktrees
  - for `PACKET_FORMAT_VERSION >= 2026-03-25`, `Done` means merge-pending PASS and `Validated (PASS)` requires recorded containment in local `main`
  - for `PACKET_FORMAT_VERSION >= 2026-03-26`, PASS closure also requires recorded `CURRENT_MAIN_COMPATIBILITY_*` truth plus `PACKET_WIDENING_DECISION=NOT_REQUIRED`; adjacent shared-surface drift must route to a follow-on or superseding packet instead of ad hoc widening
- `just external-validator-brief WP-{ID} [--json]`
  - `read-only`
  - classical/external validation contract summary
  - default text output is compact-by-default and points at the authoritative governance/code targets; use `--json` for the full machine-readable brief
- `just validator-gate-append WP-{ID} <PASS|FAIL|ABANDONED|...>`
- `just validator-gate-commit WP-{ID}`
- `just validator-gate-present WP-{ID} [verdict]`
- `just validator-gate-acknowledge WP-{ID}`
- `just validator-gate-reset WP-{ID} <confirm>`
  - `governance-write`
  - validator gate state progression/reset
  - on orchestrator-managed WPs, these write surfaces now fail early if the current branch/worktree does not resolve to a governed validator lane; use `validator-next`, `integration-validator-context-brief`, or `external-validator-brief` instead of guessing
- `just validator-gate-status WP-{ID}`
- `just validator-governance-snapshot`
- `just validator-report-structure-check`
- `just validator-phase-gate [Phase-n]`
  - `read-only`
  - governance-oriented validator status/report checks
- `just validator-scan`
- `just validator-error-codes`
- `just validator-coverage-gaps`
- `just validator-traceability`
- `just validator-hygiene-full`
  - `product-scan`
  - product-side audit and hygiene family

## Worktree and topology management

These are orchestrator/operator/topology commands, not ordinary coder commands.

- `just worktree-add WP-{ID}`
- `just coder-worktree-add WP-{ID}`
- `just wp-validator-worktree-add WP-{ID}`
- `just integration-validator-worktree-add WP-{ID}`
  - `runtime-write`
  - create/repair worktree mappings or role session worktree context
- `just backup-status`
  - `read-only`
  - backup health visibility
- `just backup-push <local_branch> <remote_branch>`
- `just backup-snapshot [label]`
  - `runtime-write`
  - safety preservation surfaces
- `just sync-all-role-worktrees`
- `just reseed-permanent-worktree-from-main <worktree_id> "<approval>"`
- `just sync-gov-to-main`
  - `governance-write`
  - topology refresh / governance propagation
- `just enumerate-cleanup-targets`
  - `read-only`
  - deterministic cleanup preview
- `just generate-worktree-cleanup-script WP-{ID} <ROLE>`
  - `read-only`
  - produce hard-bound cleanup script
- `just delete-local-worktree <worktree_id> "<approval>"`
- `just close-wp-branch WP-{ID} "<approval>" [remote]`
- `just retire-standalone-checkout <checkout_id> "<approval>"`
  - `runtime-write`
  - approved destructive/retirement surfaces

## Command-surface rules

- Prefer the role-specific wrapper when one exists.
  - Example: use `just start-wp-validator-session ...` instead of the generic `just session-start WP_VALIDATOR ...` unless you specifically need the generic form.
- `product-scan` is an alias for `validator-scan`.
- `THREAD.md` commands are not substitutes for required structured review receipts.
- A command being available in `just --list` does not mean every role may run it. Role protocols still define ownership.
- Any command not present in the current `just --list` output should be treated as retired, stale, or a documentation bug until restored.
