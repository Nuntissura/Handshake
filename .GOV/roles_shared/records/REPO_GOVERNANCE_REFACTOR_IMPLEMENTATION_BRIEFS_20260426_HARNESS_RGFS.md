# Repo Governance Refactor Implementation Briefs — Harness-Pattern Tranche

**Date:** 2026-04-26
**Authority:** `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
**Scope:** Companion implementation briefs for `RGF-242` through `RGF-249`. These items follow the closeout-canonicalization tranche (`RGF-233` through `RGF-241`) and apply runtime patterns extracted from a comparative study of four agent harnesses (Pi, Hermes, OpenClaw/ACPX, Gastown).
**Driving evidence:** `.GOV/reference/research_and_papers/harnesses/00_HARNESS_COMPARATIVE_ANALYSIS.md` and per-harness drafts `01_pi.md`, `02_hermes.md`, `03_openclaw.md`, `04_gastown.md`.
**Strategic frame:** Repo governance is a **testbed for the Handshake product**. The patterns below are architectural primitives that port to Handshake when self-hosting begins. The testbed implementation is the first proving ground; the pattern is what ultimately ships.

---

## Reading guide for fresh implementers

You are picking this up without the conversation that produced it. Each RGF brief is self-contained:

- **Problem** — what is broken now and why it costs tokens / time / correctness
- **Evidence** — concrete failure modes plus the harness pattern that informs the fix (with file paths in the cloned harness repos at in the operator's harnesses workspace, for code-level reference)
- **Breakpoints this must cover** — scenarios that the implementation must not regress on
- **Required implementation** — concrete file paths, library boundaries, and a sequenced work plan
- **Non-goals** — common misreads to avoid
- **Acceptance criteria** — the minimum bar for `DONE`

Authority surfaces you should read before changing code:

- `.GOV/codex/Handshake_Codex_v1.4.md` — repo-wide governance laws (cache-stability, role authority, mechanical-vs-judgment split)
- `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` — orchestrator authority and the no-product-code rule
- `.GOV/roles/coder/CODER_PROTOCOL.md` — coder boundary, microtask contract, worktree confinement
- `.GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md` and `.GOV/roles/integration_validator/INTEGRATION_VALIDATOR_PROTOCOL.md` — dual-track validator model
- `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md` — sanctioned command surface
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` — full RGF history and dependency graph
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_BRIEFS_20260426.md` — the closeout tranche (`RGF-233` through `RGF-241`); this tranche assumes that one is in flight or landed
- `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_BRIEFS_20260426_HARNESS_ADDENDUM.md` — research-driven addendum that names these RGFs (B.1 through B.8) and gives one-line refinements (A.1 through A.5) for the closeout tranche

Procedural rules that apply to this tranche specifically:

1. Do not start by editing protocols. Implement the mechanical reader/writer/check behavior first, then update protocols and command docs to match the new mechanics. (Same rule as the closeout tranche.)
2. Do not route deterministic governance work through ACP. Use direct `just`/node calls. (RGF-189 codified this.)
3. Cache-stability rule (introduced in this tranche by `RGF-242`): once it lands, no helper in this tranche or later may mutate an active role session's cached system prompt. Mutations land in storage; the next session sees them.
4. The role split (CODER / WP_VALIDATOR / INTEGRATION_VALIDATOR / ORCHESTRATOR) is load-bearing because the operator is a non-engineer and depends on independent verification. Do not collapse roles. The proposals here only change the *transport* of mechanical work, never the role identity.

---

## RGF-242 — Mid-Conversation Cache-Stability Policy and Ephemeral User-Message Injection

**Tag:** CORE PATTERN (ports to Handshake)
**Tier:** 1 (high impact, low effort)
**Depends on:** none structural; can land in parallel with the closeout tranche.

### Problem

Active role sessions accumulate cache invalidations whenever governance state mutates during their conversation. Today, when the orchestrator updates a WP packet, appends a receipt, syncs a dossier projection, or steers a running coder session, the change can land inside the running session's context — either in a re-built system prompt on next turn, or via re-injection of governance text the model is expected to re-read. Anthropic prefix caching is up to 75% input-token reduction; every cache miss multiplies the effective per-turn cost roughly 4×. Long WPs that touch governance state many times (closeout repair loops, repeated validator passes, dossier sync) pay this cost on every subsequent turn for the rest of the session.

This is the largest single lever on the orchestrator's input-token bill. Independent governance audit identified the orchestrator at 110M+ tokens on a single WP; cache invalidation on packet/dossier mutation is the dominant contributor.

### Evidence

Hermes Agent codifies cache-stability as a hard rule. From its `AGENTS.md:521-535` (clone at the cloned `hermes-agent` repo):

> Hermes-Agent ensures caching remains valid throughout a conversation. Do NOT implement changes that would: Alter past context mid-conversation, Change toolsets mid-conversation, Reload memories or rebuild system prompts mid-conversation. […] Slash commands that mutate system-prompt state (skills, tools, memory, etc.) must be cache-aware: default to deferred invalidation (change takes effect next session), with an opt-in `--now` flag for immediate invalidation.

Hermes injects ephemeral context into the **current turn's user message** with a `<memory-context>` fence and a system-note disclaimer telling the model to treat the content as informational, not user words (`agent/memory_manager.py:66-80`). The cached system prompt remains untouched; the model still sees the context for this turn.

Pi takes the same position by construction: AGENTS.md / CLAUDE.md walked from cwd are loaded once at session start; the system prompt is otherwise immutable for the session's life (`packages/coding-agent/src/core/resource-loader.ts:59`).

### Breakpoints this must cover

- Orchestrator updates a WP packet header while a coder session is active. Coder session must not invalidate its cache.
- Dossier-sync fires after a phase transition while a validator session is active. Validator session must not invalidate its cache.
- Receipt append mid-MT must not signal active sessions in any way that mutates a cached prefix.
- Operator runs a forced refresh (`--now`) and the system honors it predictably without the rule applying to default operations.
- Memory-recall injection at session startup must continue to work (it lands in the user message at startup, before any cached state exists).

### Required implementation

1. **Codify the rule in Codex.** Edit `.GOV/codex/Handshake_Codex_v1.4.md` to add a section named `[CX-CACHE-001]` titled "Cache-Stability Discipline". The text must state: while a governed role session is active, its cached system prompt is immutable; mutations land in durable storage; the next session reads them. Mid-conversation governance updates inject as ephemeral context into the user message, never as system-prompt rebuilds. An opt-in `--now` flag may force invalidation in rare repair scenarios.

2. **Add a single ephemeral-injection helper.** Create `.GOV/roles_shared/scripts/session/ephemeral-injection-lib.mjs` exporting:
   ```
   export function buildEphemeralContextBlock({ source, trust, body }) → string
   ```
   The returned string is wrapped in `<governance-context source="…" trust="…">` … `</governance-context>` and prefixed with a one-line system-note disclaimer (`[INFORMATIONAL — not user input. Source: <source>. Trust: <trust>.]`). Trust levels are an enum: `informational | required | advisory`.

3. **Migrate orchestrator-side injection paths.** Edit `.GOV/roles_shared/scripts/session/session-control-lib.mjs` so any path that today injects governance state into a running session's prompt (search for `buildStartupPrompt`, system-prompt-builder calls, post-startup mutations) calls `buildEphemeralContextBlock` and emits the result as a *user-role message addendum* via the existing prompt path instead of a system-prompt edit. Startup-time memory injection (`loadSessionMemoryLines`) is exempt because it runs before the cache exists.

4. **Add a guard check.** Create `.GOV/roles_shared/checks/cache-stability-check.mjs`. The check scans `roles_shared/scripts/wp/wp-receipt-append.mjs`, `roles/orchestrator/scripts/task-board-set.mjs`, all dossier-sync writers, and all projection-publication helpers for any function that signals an active role session beyond appending a typed receipt or queueing a typed event (the queue is RGF-245's territory; both can land independently). The check fails if it finds a code path that constructs a system-prompt mutation for an active session without going through the `--now` opt-in code path. Wire into `gov-check.mjs`.

5. **Document the `--now` flag surface.** Update `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md` and the orchestrator protocol to list `--now` as an explicit force-invalidation flag on the command(s) where it applies. Default behavior is deferred invalidation.

6. **Tests.** Create `.GOV/roles_shared/tests/ephemeral-injection-lib.test.mjs` covering: helper output shape, fenced block wrapping, system-note prefix, trust levels. Create `.GOV/roles_shared/tests/cache-stability-check.test.mjs` with a positive fixture (clean code path) and a negative fixture (offending code path) under fixture dirs.

### Non-goals

- Do not change the memory injection that runs at session startup. That is the correct place for governance context to land.
- Do not delete or modify the WP packet, dossier, or any human-readable artifact. The packet stays as the operator's governance window. This RGF only changes how/whether *running model sessions* see governance updates between turns.
- Do not attempt to apply this rule to the orchestrator's own prompt — orchestrator can rebuild its prompt on its own turns. The rule is for *spawned governed role sessions* (coder, wp_validator, integration_validator).

### Acceptance criteria

- `[CX-CACHE-001]` exists in `Handshake_Codex_v1.4.md` and is referenced from the orchestrator protocol.
- `buildEphemeralContextBlock` is the single helper through which mid-conversation governance context enters a running session's turn.
- `cache-stability-check` runs in `gov-check` and passes on the current codebase.
- A regression fixture proves a sequence of governance updates (packet edit + MT receipt + dossier append) does not invalidate the cached prefix of an active CODER session.
- The opt-in `--now` flag is surfaced on at least one command and documented.
- Token cost on a representative orchestrator-managed WP (run after this lands) shows a measurable input-token reduction vs. the prior baseline. Capture before/after numbers in the closeout dossier of the WP that proves it.

---

## RGF-243 — Tool-Result Audit / Model Asymmetry (`details` vs. `content`)

**Tag:** CORE PATTERN (ports to Handshake)
**Tier:** 1 (high impact, low effort)
**Depends on:** none. Compounds with RGF-242.

### Problem

Governance check outputs (phase-check, gov-check, packet-truth, validator-gate, wp-communication-health) currently return a single payload that serves both human readers and model consumers on subsequent turns. The full output enters every consumer's context, even when only a one-line verdict is needed for routing. The model pays input-tokens for verbose output it does not need for its decision; the human-facing audit also reads the same payload. Both are well served, but the model is being charged for the audit.

This is a smaller cost than RGF-242 individually, but compounds heavily with it: with RGF-242 in place, the cached prefix is stable, but turn output that re-enters the prompt next turn (tool results, verbose check output) still bloats the cost over a long WP.

### Evidence

Pi (`packages/agent/src/types.ts:291-302`, clone at the cloned `pi-mono` repo) defines `AgentToolResult` with two fields: `content` (model-visible, returned to the LLM) and `details` (UI/log only, never sent back to the model). The split is rigorous — every tool returns both. The unified diff goes in `details`; the model only sees `"Successfully replaced N block(s) in foo.ts."`. Bash output truncation paths the temp-file path to `details`; the model sees just the captured output. The asymmetry is documented in `packages/agent/README.md` and used everywhere.

This is the cleanest implementation of the audit/context split observed in any harness. It is also the most directly portable: it's a return-shape convention, not an architectural change.

### Breakpoints this must cover

- Phase-check on a long WP returns full subcheck output today. Model session must see only a one-line summary on subsequent turns.
- Gov-check bundle returns ~20 subcheck results today. Model session must see only a verdict line per subcheck.
- Operator viewport must continue to render the same level of detail by reading from the structured log.
- Dossier-sync must continue to surface the same diagnostic detail.
- A failing check must surface enough to act on without forcing the consumer to read the full payload.

### Required implementation

1. **Define the return shape.** Update `.GOV/roles_shared/scripts/lib/check-result-lib.mjs` (create if it does not exist) to export:
   ```
   export function createCheckResult({ verdict, summary, details }) → CheckResult
   ```
   Where `verdict` is `OK | WARN | FAIL`, `summary` is a string ≤ 120 characters, `details` is a structured object with arbitrary keys.

2. **Persist `details` to a structured log.** When a check is invoked through the standard runner (`phase-check`, `gov-check`, etc.), the runner appends `{check, wp_id, phase, verdict, summary, details, timestamp}` to `<wp_runtime>/<wp_id>/check_details.jsonl`. For repo-scope checks (no wp_id), append to `gov_runtime/check_details.jsonl`. Use append-only writes; never rewrite earlier entries.

3. **Restrict stdout to summary.** The check runner emits only `<verdict> | <summary>` to stdout by default. A `--verbose` flag reads from the JSONL log and emits the full details for debugging. The model-visible output is the default; humans who need detail use `--verbose`.

4. **Migrate the existing checks.** Update `.GOV/roles_shared/checks/phase-check.mjs`, `.GOV/roles_shared/checks/gov-check.mjs`, `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`, `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`, `roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`, and `roles/validator/scripts/lib/validator-governance-lib.mjs` so each check returns through `createCheckResult`. Existing inline `console.log` calls become `details` payloads.

5. **Wire dossier-sync and operator-viewport to the log.** `.GOV/roles_shared/scripts/audit/workflow-dossier.mjs` (the live-dossier sync) reads the latest `details` rows from `check_details.jsonl` for the WP and projects the rendered detail into the dossier. The operator viewport (`roles/orchestrator/scripts/operator-monitor-tui.mjs`) does the same. No model session reads the JSONL.

6. **Tests.** `.GOV/roles_shared/tests/check-result-lib.test.mjs` covers the helper. `.GOV/roles_shared/tests/check-details-log.test.mjs` covers append semantics, idempotence on duplicate writes, and reader behavior.

### Non-goals

- Do not change the diagnostic content of any check. The details payload is the same content that previously went to stdout — it just lives in the log instead.
- Do not attempt to compress the `details` payload. Storage is cheap; the goal is to keep it out of model context, not to make it small.
- Do not migrate every helper that prints. This RGF targets *checks* whose output is consumed by model sessions. Pure operator tools (operator-viewport, gov-check `--verbose`) keep their full output.

### Acceptance criteria

- `createCheckResult` is the single helper through which checks return their results.
- `check_details.jsonl` is the canonical log for full check output, and the path is documented.
- A model session running phase-check on a long WP receives a one-line summary on stdout; the full payload is in the log and accessible via `--verbose`.
- Dossier-sync output is byte-identical to the prior detail level (verify by snapshot test against a fixture WP).
- A regression fixture proves a long-running orchestrator session does not accumulate verbose check output in its context.

---

## RGF-244 — Deterministic Artifact-Malformation Absorber

**Tag:** TESTBED-WISDOM (pattern ports; catalog is testbed-specific)
**Tier:** 1 (high impact)
**Depends on:** none. Strongly compounds with RGF-238 (closeout repair loop breaker, A.4 refinement).

### Problem

Models authoring WP packets, receipts, validator reports, and dossier entries produce a recurring small set of malformations: trailing whitespace, missing trailing newline, smart-quote substitution, JSON-string-vs-array confusion, CRLF vs LF mismatch, missing or extra heading levels, `- field: value` bullet prefix on lines that should be bare `field: value`, indentation drift inside fenced blocks, and so on. Each malformation today triggers either a check failure that routes back to the model for repair (one or more turns of cost) or a workflow loop that the orchestrator drives.

`RGF-197` already proved the absorption pattern on validator reports specifically — the validator-report-structure-check parser was updated to tolerate `- ` bullet prefixes, `#### ` heading prefixes, and majority-based evidence rules. That single change reduced one fixture from 39 violations to 0. The pattern works; it has not been generalized.

### Evidence

Pi `prepareArguments` shims (`packages/coding-agent/src/core/tools/edit.ts:90-114`):

```ts
function prepareEditArguments(input: unknown): EditToolInput {
    // Some models (Opus 4.6, GLM-5.1) send edits as a JSON string instead of an array
    if (typeof args.edits === "string") {
        try {
            const parsed = JSON.parse(args.edits);
            if (Array.isArray(parsed)) args.edits = parsed;
        } catch {}
    }
    // Legacy oldText/newText fields get folded into edits[]
}
```

Pi also normalizes Unicode in fuzzy match (`packages/coding-agent/src/core/tools/edit-diff.ts:34-55`): smart quotes become straight quotes, en-dashes become hyphens, NBSP becomes space. Pi never asks the model to re-emit; it absorbs.

Hermes `coerce_tool_args()` (`model_tools.py:382`, clone at the cloned `hermes-agent` repo) does the same against JSON Schema: `"42" → 42`, `"true" → true` when the schema expects a number/bool. Saves a retry round-trip every time.

### Breakpoints this must cover

- A receipt with smart quotes is silently normalized at append time.
- A validator report with `- field: VALUE` bullet prefix is parsed correctly without failing structural check.
- A packet with CRLF line endings is normalized to LF before evaluation.
- A heading level off by one (`### Foo` where `## Foo` was expected) is normalized when the structural intent is unambiguous.
- A receipt array delivered as a JSON string is parsed back to an array.
- A trailing newline missing from a receipt does not cause append to corrupt the JSONL.
- The absorber set is observable: a log of which absorbers fired on which artifacts, so the catalog can grow with traffic.
- The absorber is *additive only*: it never rejects malformed input. Rejection remains the validator's job.

### Required implementation

1. **Catalog the modes.** Open `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`, the `RGF-197` entry in the task board, the `validator-report-structure-check.mjs` source, and recent post-work-check / packet-truth-check failure receipts. Extract the top-N malformation modes (target N = 12-15 for the first cut). For each mode, write a one-line description and a sample input/output pair. Land this catalog as a comment block at the top of the absorber index file (step 2).

2. **Implement absorber primitives.** Create `.GOV/roles_shared/scripts/lib/artifact-normalizers/index.mjs` and one file per mode under that directory. Each file exports a pure function:
   ```
   export function normalize<Mode>(input: string) → { output: string, applied: boolean, reason?: string }
   ```
   The `index.mjs` exports `runAbsorber(input, { artifactKind })` which iterates the relevant absorbers in declared order and returns `{ output, applied: [...] }`.

   Required initial absorbers:
   - `normalizeLineEndings` — CRLF → LF; preserve BOM if present (return; don't strip)
   - `normalizeTrailingNewline` — ensure exactly one trailing newline
   - `normalizeSmartQuotes` — `"“”‘’" → "\""` and `'`
   - `normalizeDashes` — em/en/hyphen-minus collapsed where the artifact is plain text (skip in fenced code blocks)
   - `normalizeJsonStringVsArray` — when a field expects an array but receives a JSON-encoded string, parse it back
   - `normalizeBulletPrefixedFields` — `- field: value` → `field: value` when the artifact spec is bare key-value
   - `normalizeHeadingPrefix` — `#### LABEL:` → `LABEL:` when the structural spec is bare key-value
   - `normalizeFieldValueWhitespace` — trim trailing whitespace on key-value lines without altering fenced content
   - `normalizeWindowsPathEscapes` — backslash-escape collapse for paths in JSON
   - `normalizeNullishFieldValues` — `field: None` / `field: null` / `field: NULL` → `field: ` when spec allows omission

3. **Wire absorbers into write paths.** Update:
   - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs` — call `runAbsorber(payload, { artifactKind: "receipt" })` before persist; record the `applied` list as a metadata field on the persisted receipt.
   - `roles/validator/checks/validator-report-structure-check.mjs` — call before evaluation; the existing `RGF-197` tolerances graduate from inline regex into named absorbers.
   - `.GOV/roles_shared/checks/packet-truth-check.mjs` (and any other packet-evaluation check) — same.
   - The closeout-repair pre-pass (per addendum A.4 / `RGF-238`) — first stage is "run absorbers; if any applied, re-evaluate; only then begin the repair budget".

4. **Hit-count log.** Each invocation of `runAbsorber` appends `{timestamp, artifactKind, wp_id?, applied: [...]}` to `gov_runtime/absorber_hits.jsonl`. A weekly review (operator-driven, not automated) inspects this log to identify new modes worth absorbing.

5. **Tests.** One fixture file per absorber under `.GOV/roles_shared/tests/artifact-normalizers/` with before/after samples and `applied: true/false` assertions.

### Non-goals

- Do not delete the structural validation in checks. Absorbers run *before* validation; validation is what catches the malformations the absorber set does not yet cover.
- Do not absorb anything that changes semantic meaning. If an absorber would alter the structural intent (e.g., turn a non-empty value into an empty value), the absorber must not fire.
- Do not chain absorbers in ways that re-introduce a malformation. Order matters; tests must cover the ordering.
- Do not absorb inside fenced code blocks unless the absorber is explicitly safe inside code (line endings, trailing newline). Smart-quote and dash normalization stops at fence boundaries.

### Acceptance criteria

- The absorber index runs in declared order and returns the normalized output plus a list of applied absorbers.
- All write paths listed above invoke the absorber suite before persist/evaluation.
- The absorber-hits log accumulates real entries within one WP cycle of deployment.
- A regression suite covers each absorber with at least one before/after fixture.
- A WP that previously hit one of the cataloged malformations runs end-to-end without triggering the malformation's downstream repair path.
- The dossier-sync continues to render WP closeout normally; absorber output is invisible to operator-facing surfaces.

---

## RGF-245 — Turn-Boundary Nudge Queue with Atomic Claim

**Tag:** CORE PATTERN (ports to Handshake)
**Tier:** 2 (architectural)
**Depends on:** RGF-242 (cache-stability) is recommended but not strictly required.

### Problem

Orchestrator and watchdog re-steering currently signals running role sessions through ACP `SEND_PROMPT` paths that can interrupt mid-tool-call, get lost during heavy host load, or arrive while the session is in a state that cannot legally consume the steering message. `RGF-30` (event-driven happy-path relay automation) and `RGF-166` (non-LLM relay watchdog) cover the wake-up *semantics* but not the on-disk *delivery primitive*. The result is occasional duplicate sends, occasional missed nudges, and the well-documented governance complaint that the orchestrator polls for results when it should not.

### Evidence

Gastown `internal/nudge/queue.go` (clone at the cloned `gastown` repo):

> The nudge queue allows messages to be delivered cooperatively: instead of sending text directly to a tmux session (which cancels in-flight tool calls), nudges are written to a queue directory and picked up by the agent's UserPromptSubmit hook at the next natural turn boundary.

The mechanics:
- Queue dir per session: `<townRoot>/.runtime/nudge_queue/<session>/`
- Each nudge is a JSON file named `<unix-nano>-<random-hex>.json` for FIFO ordering with collision-resistance
- Drain uses **rename-then-process**: each file is atomically renamed to a `.claimed` suffix before reading, so concurrent drainers can't double-deliver
- Stale `.claimed` files older than 5 minutes from crashed drainers are *renamed back* to `.json`, not deleted, so the nudge survives drainer crashes
- TTLs: 30 min for normal, 2h for urgent
- Hard cap: 50 entries per session; enqueue returns an error rather than dropping silently
- Failed deliveries get **Requeue** with original timestamps preserved

Gastown CHANGELOG 1.0.1 (2026-04-25) documents that they hit the same context-bloat-from-accumulating-mail bug Handshake reports, and shipped this exact mechanism plus archive-on-done as the fix. Verbatim relevance.

### Breakpoints this must cover

- A nudge enqueued during an active tool call is delivered at the next natural prompt boundary, not by interrupting the call.
- Two concurrent drainers cannot double-deliver the same nudge.
- A drainer crash mid-claim does not lose the nudge; the orphan is recovered automatically after 5 minutes.
- A nudge older than its TTL is silently expired and not delivered.
- A queue depth exceeding `MaxQueueDepth` returns an explicit error rather than silently dropping.
- A failed delivery (e.g., session was actually dead) re-queues with original timestamp, preserving FIFO order.
- Emergency RED_ALERT cases (RGF-230) bypass the queue and use direct stdin (preserve existing behavior).

### Required implementation

1. **Library.** Create `.GOV/roles_shared/scripts/session/nudge-queue-lib.mjs` exporting:
   ```
   export function enqueueNudge({ sessionId, payload, ttl = 1800, priority = "normal" }) → { ok, queueDepth, error? }
   export function drainNudges({ sessionId, drainerId }) → { nudges: [...], orphansRecovered: N }
   export function listQueueDepth(sessionId) → number
   export function expirePastTtl(sessionId) → number  // returns count expired
   ```

2. **Storage layout.** Per-session queue dir at `gov_runtime/nudges/<wp_id>/<session_id>/`. Filename `<unix_nano>-<random_hex>.json`. Each file's body is the typed payload (see step 3). Constants: `MaxQueueDepth = 50`, `OrphanRecoveryAgeMs = 5 * 60 * 1000`, `DefaultTtlSec = 1800`, `UrgentTtlSec = 7200`.

3. **Typed payload.** Schema `.GOV/roles_shared/schemas/NUDGE_PAYLOAD.schema.json`:
   ```
   {
     "kind": "STEER" | "RELAUNCH_REQUEST" | "PHASE_TRANSITION" | "MT_VERDICT" | "GOVERNANCE_REMINDER",
     "from_role": "ORCHESTRATOR" | "WP_VALIDATOR" | "RELAY_WATCHDOG" | …,
     "wp_id": string,
     "correlation_id": string,
     "body": object,
     "enqueued_at": ISO8601,
     "expires_at": ISO8601,
     "priority": "normal" | "urgent",
     "delivery_attempts": number
   }
   ```
   Validate against the schema on enqueue.

4. **Drain on the consumer side.** Each spawned governed session's startup hook (per `RGF-246` — implement the hook surface here even if `RGF-246` lands separately) drains its queue at the start of each turn:
   - Find all `*.json` files in queue dir; sort by filename (FIFO).
   - For each: atomically `rename(*.json → *.claimed)`. If the rename fails (another drainer claimed it), skip. Read the claimed file. Append the payload to the upcoming user-message turn (using the RGF-242 ephemeral injection helper). On successful append, `unlink(*.claimed)`. On failure, `rename(*.claimed → *.json)` to requeue.
   - Recover orphans: any `*.claimed` older than `OrphanRecoveryAgeMs` is renamed back to `*.json`.

5. **Migrate signaling paths.** Update:
   - `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs` — for non-emergency steers, enqueue via `nudge-queue-lib` instead of direct ACP `SEND_PROMPT`. Emergency cases (RED_ALERT, operator force) keep direct path.
   - `.GOV/roles/orchestrator/scripts/wp-relay-watchdog.mjs` — same migration. The watchdog's role becomes "drop a nudge and observe", not "send and wait".
   - `.GOV/roles_shared/scripts/lib/wp-relay-escalation-lib.mjs` — escalation produces a nudge with `priority: "urgent"` and a longer TTL.

6. **Operator visibility.** Operator-viewport (`roles/orchestrator/scripts/operator-monitor-tui.mjs`) gains a per-session nudge-queue panel showing depth, oldest pending TTL, and recently delivered nudges. `orchestrator-health` reports queue depth as a health signal.

7. **Tests.**
   - `.GOV/roles_shared/tests/nudge-queue-lib.test.mjs` — unit tests for enqueue, drain, FIFO, TTL expiry, depth cap.
   - Concurrency fixture: two simulated drainers race; assert exactly-once delivery.
   - Crash fixture: drainer crashes mid-claim (simulate by killing process between rename and consume); orphan recovery restores the nudge.
   - End-to-end fixture: orchestrator enqueues a STEER while a coder session is in a simulated tool-call; nudge is delivered after the tool call completes, not during.

### Non-goals

- Do not delete the direct ACP `SEND_PROMPT` path. Emergency steering and operator-forced messages keep using it.
- Do not attempt cross-session broadcast through this primitive. One queue, one consumer.
- Do not migrate the broker's pending_control_queue (RGF-206). That queue is for ACP-side ingress; this is for governance-side delivery. Different layer.

### Acceptance criteria

- All listed library functions exist and have tests.
- The non-emergency path through `orchestrator-steer-next` and `wp-relay-watchdog` enqueues rather than direct-sends.
- Concurrency fixture passes with exactly-once delivery.
- Orphan-recovery fixture passes.
- A WP run after this lands shows orchestrator session token cost dropping (no more re-steering during cache-warm windows). Capture before/after in dossier.

---

## RGF-246 — Hook-Driven Session Self-Rehydration

**Tag:** CORE PATTERN (ports to Handshake)
**Tier:** 2 (architectural)
**Depends on:** RGF-245 (nudge-queue drain runs in the same hook).

### Problem

The orchestrator currently constructs the prompt for every spawned governed role session — coder, wp_validator, integration_validator. Construction includes governance state (current packet, MT identity, work assignment), authority surface (protocol references, codex anchors), memory injection, and per-WP context. This makes the orchestrator (a) a bottleneck on every relaunch, (b) responsible for context completeness ("did the orchestrator remember to include the MT contract?"), and (c) the primary turn-cost contributor on simple WP transitions.

### Evidence

Gastown `gt prime` runs in the agent's own `SessionStart` hook (`internal/hooks/templates/claude/settings-autonomous.json:100-110`):

```json
{
  "SessionStart": [
    { "hooks": [
      { "type": "command", "command": "gt prime --hook && gt mail check --inject" }
    ]}
  ]
}
```

The agent self-rehydrates from beads (their structured state store) when its session starts. The orchestrator only dispatches a tiny directive — which WP, which role, which task. Every session start is identical regardless of which orchestrator instance dispatched it: the canonical truth lives in storage, not in the dispatcher's prompt-construction code.

Pi has a similar reduction: AGENTS.md / CLAUDE.md walked from cwd are loaded once at session start (`packages/coding-agent/src/core/resource-loader.ts:59`); there is no orchestrator constructing context.

### Breakpoints this must cover

- A CODER session spawned via the new hook builds its own prompt from canonical truth (terminal record, packet projection, MT board, memory) without the orchestrator providing context.
- A CODER session resumed after compaction self-rehydrates without orchestrator intervention.
- A CODER session resumed after process restart (crash recovery) self-rehydrates from canonical truth, not from any cached orchestrator state.
- WP_VALIDATOR sessions self-rehydrate analogously.
- INTEGRATION_VALIDATOR sessions self-rehydrate analogously, with the kernel-vs-main authority rules from `RGF-41` preserved.
- Orchestrator continues to construct its own prompt (the orchestrator is not subject to this hook surface; the rule applies to spawned governed roles only).
- Operator can override the hook-built prompt via an explicit `--inline-prompt` flag for repair scenarios.

### Required implementation

1. **Define the hook surface.** Each governed role gets a `role-self-prime` script. Create `.GOV/roles_shared/scripts/session/role-self-prime.mjs` exporting:
   ```
   export async function rolePrime({ role, wpId, mtId?, sessionId }) → string
   ```
   The function reads canonical truth (in priority order: terminal closeout record from RGF-233, current packet projection, MT board, runtime status, repomem snapshot, governance memory) and assembles the system prompt. The output replaces the existing per-role prompt-builder code.

2. **Migrate per-role assembly.** Today, `.GOV/roles_shared/scripts/session/session-control-lib.mjs` has a `buildStartupPrompt` function that the orchestrator calls. Refactor:
   - Extract the role-specific assembly logic into `rolePrime`.
   - `buildStartupPrompt` becomes a thin wrapper that, for spawned governed roles, returns a stub prompt referencing the hook command. For the orchestrator's own prompt (and the rescue-orchestrator path), it continues to assemble inline.

3. **Configure provider hooks.** For each provider (Claude Code, Codex, Cursor, Gemini, Ollama profiles), add hook configuration that invokes `role-self-prime` on session start and on pre-compaction. Files to update:
   - `.GOV/hooks/templates/claude/settings-autonomous.json`
   - `.GOV/hooks/templates/codex/settings-autonomous.json`
   - similar for cursor / gemini / ollama-resident profiles
   The hook command form (Claude example): `node .GOV/roles_shared/scripts/session/role-self-prime.mjs --role CODER --wp-id ${WP_ID} --mt-id ${MT_ID} --session-id ${SESSION_ID}`. Environment variables come from the launch context.

4. **Compaction self-recover.** Add a `PreCompact` hook that re-runs `role-self-prime` and writes the result into the compaction summary. After compaction, the new effective prompt prefix is the freshly primed prompt.

5. **Operator override.** Add a `--inline-prompt` flag to `roles/orchestrator/scripts/launch-cli-session.mjs` for the rare case where an operator must hand-construct a prompt for a repair scenario. Default behavior is hook-driven.

6. **Migrate dispatch to a tiny directive.** `launch-cli-session.mjs` for governed roles dispatches `{wpId, mtId, role, sessionId}` plus the model profile. It does not construct content. The hook does.

7. **Tests.**
   - `.GOV/roles_shared/tests/role-self-prime.test.mjs` covering each role's prime output against fixture WPs.
   - End-to-end fixture: spawn a CODER session via the new hook; assert the prompt content matches what the legacy orchestrator-construction path produced for the same WP.
   - Compaction fixture: simulate a compaction event; assert the post-compaction prompt is hook-built and equivalent to the pre-compaction state.

### Non-goals

- Do not change the role identity or authority. The hook just changes who *constructs* the prompt; the role's protocol authority is unchanged.
- Do not touch the orchestrator's own prompt construction. The orchestrator stays inline.
- Do not delete the legacy inline path; it remains as the `--inline-prompt` operator escape hatch.
- Do not move memory injection out of the prompt path. Memory still lands in the prompt; it just lands via the hook.

### Acceptance criteria

- Every spawned governed role session prompt is built by the hook, not by orchestrator-side code.
- The orchestrator's per-WP turn count drops measurably on the first WP run after this lands (capture in dossier).
- Compaction events leave the resumed session with a freshly primed prompt.
- The `--inline-prompt` escape hatch exists and is documented.
- No regression in role authority — orchestrator protocol checks still pass.

---

## RGF-247 — Mechanical-Track Validator-as-Tool-Result

**Tag:** CORE PATTERN (ports to Handshake)
**Tier:** 3 (deeper)
**Depends on:** RGF-242, RGF-243, RGF-246. Interacts closely with `RGF-79` (dual-track validator) and `RGF-190` (WP_VALIDATOR per-MT protocol).

### Problem

Per-MT validation currently routes through ACP to a separate WP_VALIDATOR session for both the **mechanical track** (boundary check, scope containment, file-list match against MT contract, build-pass proof) and the **judgment track** (Master Spec primitive retention, code review, anti-vibe). The round-trip cost is: orchestrator turn → ACP launch (or steer) → validator reads packet + receipts + diff → emits verdict → orchestrator reads verdict → orchestrator updates dossier → next turn. Each "→" is a model call. The mechanical track does not require model judgment for most MTs — it is checks against a contract.

The dual-track model (`RGF-79`, `RGF-191`) recognized this distinction. This RGF carries it through to the transport layer: mechanical-track verdicts become a synchronous helper invocation; judgment-track verdicts remain a separate ACP session.

### Evidence

Pi `afterToolCall` hook + `completeSimple()` inline (`packages/agent/src/types.ts:75-101`, `packages/coding-agent/src/core/compaction/compaction.ts:574-578`): Pi's compaction calls the same model provider for summarization inline, inside the host process, returning a typed result. The pattern is "if the work is mechanical or scoped, do it inline; only spawn a session when the work needs full agent loop semantics".

The dual-track distinction itself is repo-side: `RGF-79` (mechanical track verdict + spec retention track verdict, both required for PASS).

### Breakpoints this must cover

- Per-MT mechanical verdicts (boundary, scope, file-list, build-pass) come back without spawning a WP_VALIDATOR ACP session.
- Per-MT judgment verdicts (spec retention, code review) continue to spawn a separate ACP WP_VALIDATOR session — this RGF does not change them.
- A mechanical FAIL routes the coder session into immediate remediation without orchestrator mediation.
- A mechanical PASS does not authorize PASS-grade closeout on its own; both tracks remain required (RGF-79 invariant).
- The mechanical helper writes a typed receipt (`MT_VERDICT_MECHANICAL`) that downstream routing reads.
- Crash mid-mechanical-check is recoverable; the helper is idempotent on the same MT.
- Operator can force the legacy path (mechanical track via ACP) if needed for repair.

### Required implementation

1. **Define the mechanical verdict shape.** Update `roles_shared/schemas/WP_RECEIPT.schema.json` to add `MT_VERDICT_MECHANICAL` as a receipt kind with body fields: `mt_id`, `verdict (PASS | FAIL)`, `concerns: [{key, severity, evidence_path}]`, `boundary_check_result`, `scope_check_result`, `file_list_match_result`, `build_pass_evidence`, `helper_invocation_id`.

2. **Implement the mechanical helper.** Create `.GOV/roles/wp_validator/scripts/wp-validator-mechanical-track.mjs` exporting a CLI plus a callable function:
   ```
   export async function runMechanicalTrack({ wpId, mtId, range }) → MechanicalResult
   ```
   The function performs:
   - Worktree confinement check (current head, file list, scope match)
   - File-list match against MT contract
   - Boundary check (no edits outside declared MT files)
   - Build-pass evidence ingest (consume the existing build artifact from the coder's post-commit hook per `RGF-98`)
   - Receipt structural check (run absorbers from RGF-244, then validate)
   Returns the typed result. Writes a `MT_VERDICT_MECHANICAL` receipt as a side effect.

3. **Wire into the coder session lifecycle.** Update the post-commit hook (per `RGF-98` / `RGF-106`) so completing an MT triggers `wp-validator-mechanical-track` *as a synchronous tool call within the coder session*, not as an ACP launch. The result returns into the coder turn as a tool result with the typed mechanical verdict in `details` (RGF-243) and a one-line summary in `content`.

4. **Routing.** Update `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs` so the mechanical receipt projects into the route anchor as `MT_MECHANICAL_PASS` or `MT_MECHANICAL_FAIL`. The downstream judgment-track WP_VALIDATOR ACP session reads the mechanical receipt as input rather than re-deriving the same checks.

5. **Preserve dual-track invariant.** Update `.GOV/roles_shared/scripts/lib/computed-policy-gate-lib.mjs` so PASS-grade closeout requires both `MT_VERDICT_MECHANICAL: PASS` and `MT_VERDICT_JUDGMENT: PASS` for every MT (and an aggregate WP-level judgment from the integration validator). A mechanical PASS without a judgment PASS does not authorize closeout.

6. **Operator escape hatch.** Add a `--legacy-acp-mechanical` flag to the post-commit hook for the rare case where mechanical-track must run as an ACP session (debug, A/B comparison, or operator preference). Default is inline.

7. **Tests.**
   - `.GOV/roles/wp_validator/tests/wp-validator-mechanical-track.test.mjs` — unit tests for each check, then end-to-end on a fixture MT.
   - Regression: a fixture WP runs to PASS using only inline mechanical track + judgment-track ACP. Capture run-time and turn count vs. legacy.
   - Negative fixture: a clear scope violation (file edited outside MT contract) routes to coder remediation immediately, without spawning a WP_VALIDATOR session.

### Non-goals

- Do not collapse the dual-track verdict into a single verdict. Both tracks remain required for PASS.
- Do not remove the WP_VALIDATOR ACP session. It still runs the judgment track. This RGF only changes the *transport* of the mechanical track.
- Do not change the boundary-enforcement contract from `RGF-195` (worktree confinement). That stays mechanical and authoritative.
- Do not change the per-MT validator's escalation behavior on judgment-track FAIL. Same role, same authority.

### Acceptance criteria

- Per-MT mechanical verdicts come back inline without an ACP launch.
- A clean WP run shows ~50% reduction in WP_VALIDATOR ACP session spawn count (only judgment-track spawns remain).
- A regression fixture proves mechanical-track FAIL routes to immediate coder remediation.
- The dual-track invariant is enforced (no PASS without both tracks).
- Operator escape hatch exists and is documented.

---

## RGF-248 — Named-Verb Inter-Role Message Schema

**Tag:** CORE PATTERN (ports to Handshake)
**Tier:** 2 (architectural)
**Depends on:** RGF-244 (absorber). Compounds with RGF-245 (nudge queue payloads use the same schema).

### Problem

Per-MT and per-WP inter-role communication still flows through prose receipts that the model authors and that other roles read. The closeout canonicalization tranche (RGF-233) addresses terminal-event truth; per-MT events remain prose. The cost is: every receipt is a model authoring task; every read is parsing prose; malformations require repair turns.

The `RGF-205` governed action envelope is the closest existing primitive — it covers session-control actions. This RGF extends the same shape to inter-role traffic that lives in receipts.

### Evidence

Gastown mail protocol (`docs/design/mail-protocol.md`): named verbs (POLECAT_DONE, MERGE_READY, MERGED, MERGE_FAILED, REWORK_REQUEST), each with a fixed Subject (e.g. `POLECAT_DONE <polecat-name>`) and a fixed Body schema (3-5 labelled key-value lines), a defined Route, and a defined Handler. The schema is small enough that malformation is rare; the parser is unambiguous.

OpenClaw-ACPX `QueueSubmitRequest` / `QueueCancelRequest` / `QueueSetModeRequest` (`src/cli/queue/messages.ts:24-120`, clone at the cloned `openclaw-acpx` repo): typed JSON-RPC messages, generation-numbered, schema-validated. Same pattern at the wire.

### Breakpoints this must cover

- A WP run can complete using only verb-typed receipts.
- A receipt with prose body (legacy form) continues to work for backward compatibility.
- Receipt-driven routing (`RGF-200`) reads verb fields directly without parsing prose.
- The dossier-projection materializes verb receipts into prose for human reading (the operator never loses the human-readable view).
- Schema validation runs on enqueue (write-time), not just at read.
- A new verb is a structural change reviewed under RGF process; ad-hoc verb introduction fails closed.

### Required implementation

1. **Define the verb set.** Initial verbs (target ~6-8 to keep the surface small):
   - `MT_HANDOFF` — coder → wp_validator. Body: `mt_id`, `range`, `commit`, `summary`.
   - `MT_VERDICT` — wp_validator → coder/orchestrator. Body: `mt_id`, `verdict (PASS|FAIL)`, `concerns[]`, `track (MECHANICAL|JUDGMENT)`.
   - `MT_REMEDIATION_REQUIRED` — wp_validator → coder. Body: `mt_id`, `concerns[]`, `next_action`.
   - `WP_HANDOFF` — coder → orchestrator (full WP completion). Body: `wp_id`, `final_range`, `mts_completed[]`, `summary`.
   - `INTEGRATION_VERDICT` — integration_validator → orchestrator. Body: `wp_id`, `verdict`, `mechanical_track`, `judgment_track`, `closeout_path`.
   - `CONCERN` — any role → orchestrator. Body: `concern_class`, `severity`, `wp_id?`, `mt_id?`, `evidence_path`, `notes`.
   - `PHASE_TRANSITION` — orchestrator → all observers. Body: `wp_id`, `from_phase`, `to_phase`, `provenance`.
   - `RELAUNCH_REQUEST` — orchestrator → role. Body: `wp_id`, `target_role`, `reason`, `priority`.

2. **Schema files.** One JSON Schema per verb under `.GOV/roles_shared/schemas/inter_role_verbs/`:
   - `MT_HANDOFF.schema.json`
   - `MT_VERDICT.schema.json`
   - `MT_REMEDIATION_REQUIRED.schema.json`
   - `WP_HANDOFF.schema.json`
   - `INTEGRATION_VERDICT.schema.json`
   - `CONCERN.schema.json`
   - `PHASE_TRANSITION.schema.json`
   - `RELAUNCH_REQUEST.schema.json`

3. **Receipt-append integration.** Update `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs` to accept `--verb <NAME>` and validate the body against the corresponding schema. Verb-typed receipts persist with a `verb` field. Non-verb receipts continue to work and persist as `verb: null` (legacy).

4. **Routing-layer reader.** Update `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs` and `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs` to read verb fields directly. When verb is null, fall back to the existing prose parser.

5. **Dossier-projection writer.** Update `.GOV/roles_shared/scripts/audit/workflow-dossier-lib.mjs` to render verb receipts into the human-readable dossier sections. Each verb gets a small render template (e.g., `MT_VERDICT` becomes a one-line entry with the verdict + concerns list). The operator continues to see the same level of detail; the wire format underneath is structured.

6. **Nudge-queue reuse.** RGF-245 nudge payloads use the same verb schemas where applicable (especially `MT_VERDICT`, `RELAUNCH_REQUEST`, `PHASE_TRANSITION`, `CONCERN`). Any nudge with a verb body is schema-validated on enqueue.

7. **Migration discipline.** Update protocol surfaces (`CODER_PROTOCOL.md`, `WP_VALIDATOR_PROTOCOL.md`, `INTEGRATION_VALIDATOR_PROTOCOL.md`, `ORCHESTRATOR_PROTOCOL.md`) to specify which verbs each role emits and consumes. Add a `verb-coverage-check` to `gov-check` that scans recent WPs and flags any role-pair traffic still flowing as prose receipts (informational only at first; promotion to fail-closed comes after migration completes on a few WPs).

8. **Tests.**
   - One schema fixture per verb under `.GOV/roles_shared/tests/inter_role_verbs/`.
   - End-to-end fixture: a WP that runs entirely through verb receipts. Assert dossier rendering matches a snapshot.
   - Backward-compatibility fixture: a WP that mixes verb and legacy prose receipts; both produce correct routing.

### Non-goals

- Do not delete the human-readable dossier projection. Operators read prose; verbs are the wire.
- Do not introduce a verb for every conceivable inter-role communication. Start with 6-8; grow only with evidence.
- Do not replace `RGF-205` governed action envelopes. Those cover session-control actions; verbs cover inter-role receipts. Different domains.
- Do not collapse roles. The role split is preserved; verbs are about how roles talk, not about who they are.

### Acceptance criteria

- Eight schemas exist with tests.
- `wp-receipt-append --verb` works and validates bodies.
- A pilot WP runs end-to-end using only verb receipts and produces a dossier byte-equivalent to the legacy form (modulo cosmetic differences).
- The verb-coverage check reports verb adoption percentage per role; the trend rises across WPs after rollout.
- No regression in routing or dossier rendering for legacy prose receipts.

---

## RGF-249 — Predecessor-Session Lookup for Compaction and Restart

**Tag:** CORE PATTERN (ports to Handshake)
**Tier:** 3 (deeper)
**Depends on:** RGF-246 (the self-prime hook is the natural place to invoke the lookup).

### Problem

When a session compacts or restarts (process crash, broker restart, host reboot), it currently re-reads WP/MT documents and packet history to reconstruct context. This is partially redundant with the memory-injection path (RGF-115 through RGF-147) but governance-event-specific context (recent receipts, recent steers, recent verdict transitions) is not in the memory store today. The result is high re-entry cost for long WPs.

### Evidence

Gastown `gt seance` (`internal/cmd/seance.go` and related): agents query their predecessor session's `.events.jsonl` to recover context without re-reading the codebase. The events log is small (~1KB per turn for structured events) and the summary is bounded.

### Breakpoints this must cover

- A compacted CODER session resumes without re-reading the WP packet, validator reports, or MT board on its first turn.
- A restarted session (process crash) self-rehydrates from the predecessor's events log + canonical truth, not from any cached orchestrator state.
- The predecessor summary fits within a 500-token budget for normal WP lengths.
- A session with no predecessor (first session of a WP) skips the predecessor-summary section gracefully.
- Multiple predecessor candidates (e.g., the same role had two prior sessions) resolve to the most recent.
- Predecessor summary respects role-specific boundaries: a CODER session reads the prior CODER session's events, not a WP_VALIDATOR session's events.

### Required implementation

1. **Events log writer.** Every governed session writes a structured `events.jsonl` with one row per significant event:
   - Tool calls (name, args summary, result class, duration)
   - Receipts emitted (kind, verb, mt_id)
   - Files touched (path, action: read|write|edit)
   - MT progression (mt_id, transition)
   - Verdict transitions (kind, from, to)
   Storage path: `gov_runtime/<wp_id>/sessions/<session_id>/events.jsonl`. Use append-only writes.

2. **Lookup helper.** Create `.GOV/roles_shared/scripts/session/predecessor-lookup-lib.mjs` exporting:
   ```
   export async function getPredecessorSummary({ wpId, role, currentSessionId, tokenBudget = 500 }) → string | null
   ```
   The function:
   - Lists prior sessions for `(wpId, role)` from session registry, ordered by close time desc.
   - Picks the most recent.
   - Reads its `events.jsonl`, summarizes into structured prose under the token budget (last 10 tool calls, last 5 receipts, last 3 file edits, last 2 verdict transitions, last steer received).
   - Returns the summary text wrapped in a `<predecessor-summary>` fence.

3. **Wire into self-prime.** RGF-246's `rolePrime` calls `getPredecessorSummary` and includes the result in the assembled prompt's user-message addendum (per RGF-242 ephemeral injection). When the session is the first for the role, the addendum is omitted.

4. **Compaction integration.** The `PreCompact` hook (RGF-246 step 4) re-runs `getPredecessorSummary` against the current session's events log (the "predecessor" of the post-compaction continuation is the pre-compaction session itself).

5. **Tests.**
   - `.GOV/roles_shared/tests/predecessor-lookup-lib.test.mjs` — unit tests for fixture event logs.
   - End-to-end fixture: spawn a CODER session, advance through 10 turns, simulate compaction, assert the post-compaction session's prompt includes the predecessor summary and the summary is within the token budget.
   - Empty-predecessor fixture: first session for a role; assert the addendum is omitted gracefully.

### Non-goals

- Do not replace the memory-injection path. Predecessor lookup is event-log-specific; memory injection is governance-context-wide. They run in parallel.
- Do not include verbose tool output in the predecessor summary. Summaries cite tool calls by name + result class, not full payloads.
- Do not cross role boundaries. A CODER session does not see a WP_VALIDATOR session's events. Cross-role context comes through receipts and verb messages.

### Acceptance criteria

- Every governed session writes `events.jsonl`.
- `getPredecessorSummary` returns a bounded, token-budgeted summary.
- A compaction fixture proves the resumed session reads the predecessor summary and recovers context without re-reading WP documents.
- A first-session fixture proves the empty-predecessor path works gracefully.
- Re-entry token cost on a long-WP fixture (post-compaction) drops measurably vs. the legacy path.

---

## Suggested implementation order

This sequencing balances payback timing, dependency order, and operator confidence-building. Each step ships independently and produces measurable signal.

1. **`RGF-242` — Cache-stability policy.** Largest single token-cost reduction available. Land first; measure on the next WP.
2. **`RGF-243` — Tool-result asymmetry.** Compounds with RGF-242. Land in parallel or right after.
3. **`RGF-244` — Artifact-malformation absorber.** Cuts repair-turn cost. Independent; can land in parallel with the above.
4. **`RGF-245` — Nudge queue.** Architectural; solves polling waste and mid-stream-interruption. Lands after Tier 1 to consume the cache-stable, asymmetric foundation.
5. **`RGF-246` — Hook self-rehydration.** Pairs naturally with RGF-245 (drain runs in the same hook). Removes orchestrator-as-bottleneck on relaunches.
6. **`RGF-248` — Named verbs.** Compounds with RGF-244 (verbs are small enough that absorption rarely fires). Migrate one WP first; expand.
7. **`RGF-247` — Mechanical-track validator-as-tool-result.** Bigger lift. Lands after RGF-242/243/246 are stable so the inline path is well-supported.
8. **`RGF-249` — Predecessor-session lookup.** Lands last. Useful primarily on long-WP scenarios; depends on RGF-246's hook surface.

## Fresh-model starting points

When you pick this up:

- Read the synthesis `.GOV/reference/research_and_papers/harnesses/00_HARNESS_COMPARATIVE_ANALYSIS.md` first — it explains *why* these patterns exist, with cross-harness evidence.
- Read the per-harness draft most relevant to your current RGF (e.g., `02_hermes.md` for RGF-242, `04_gastown.md` for RGF-245, `01_pi.md` for RGF-243).
- Read the addendum `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_BRIEFS_20260426_HARNESS_ADDENDUM.md` for the strategic frame and the cross-cutting analysis.
- Inspect the cloned harnesses (in the operator's harnesses workspace) for code-level reference. Code citations in this brief use repo-relative paths inside those clones.
- Before changing protocol text, implement the mechanical reader/writer/check behavior. Protocol updates follow code, not lead it.
- Do not route deterministic governance work through ACP. Direct `just`/node calls only.
- Cache-stability (RGF-242) is the foundation; once it lands, every later RGF must respect it.
- The role split is load-bearing. The proposals here change *transport* and *prompt construction*, never role identity.

## Reporting back

On completion of each RGF:
- Append a row to `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md` (status `DONE`, exit signal sentence, primary surfaces list).
- Append a CHANGELOG entry under the appropriate `GOV-CHANGE-<date>-<seq>` ID.
- If the RGF produced a measurable token-cost or run-time change, capture the before/after numbers in the closeout dossier of the WP that proves it.
- If you discover a malformation mode RGF-244's catalog does not cover, add an absorber for it and update the catalog. The absorber set is meant to grow.
