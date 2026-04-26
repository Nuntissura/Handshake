# Repo Governance Refactor — Harness-Research Addendum

**Date:** 2026-04-26
**Companion to:** `REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_BRIEFS_20260426.md`
**Driving evidence:** `.GOV/reference/research_and_papers/harnesses/00_HARNESS_COMPARATIVE_ANALYSIS.md` and the four per-harness drafts (Pi, Hermes, OpenClaw/ACPX, Gastown)
**Scope:** Read against the QUEUED tranche `RGF-233` through `RGF-241` and the broader task-board state. Identifies (A) refinements to 233-241 the harness research suggests, (B) new RGF candidates not yet on the board, and (C) DONE items that may merit a follow-on review against the research patterns.

---

## Strategic frame: testbed-first, with explicit migration target

Handshake's repo governance is a **testbed for the Handshake product**, not the long-term governance surface. The product currently carries an older snapshot of the repo governance baked in. The plan is: stabilize current repo governance and workflow until the brittleness is resolved sufficiently, then port the validated patterns into the Handshake product, then stop tweaking repo governance and start tweaking Handshake's own governance directly — Handshake building Handshake.

This shapes how to evaluate the proposals below. The work that pays back twice — once on the testbed and again when ported — is architectural primitives that are not coupled to `.GOV/` layout, justfile shape, or any repo-specific surface. Cache-stability policy, tool-output asymmetry, nudge queues, hook self-rehydration, named-verb wire schemas, predecessor-session lookup — all of these are runtime patterns that map directly into Handshake when self-hosting begins. Testbed-specific work (catalog of malformation modes, terminal-record schema fields, specific check wiring) is still worth doing because it stabilizes the testbed, but the *pattern* ports while the *artifact* gets rebuilt against Handshake's runtime.

The proposals below are tagged informally as **CORE PATTERN (ports to Handshake)** vs. **TESTBED-WISDOM (informs Handshake, artifact rebuilt)** so when migration begins it is clear which lessons travel. The Tier 1/2/3 sequencing already prioritises core patterns first.

---

## How to read this addendum

The four harnesses (Pi, Hermes, OpenClaw/ACPX, Gastown) converge on the same diagnosis Handshake's parallel governance audit reached independently: state-in-documents is the dominant cost driver. The QUEUED 233-241 tranche addresses one specific surface — **closeout canonicalization** — and is well-shaped for that. Most of the refinements below are not "the tranche is wrong" but rather "the tranche is necessary and the principle generalizes; here's how to extend it past closeout".

The board already covers many of the harness lessons (RGF-101, RGF-150, RGF-189, RGF-200, RGF-204, RGF-205 in particular). What remains undone is mostly:
1. Generalizing the single-writer / typed-event pattern past closeout to per-MT communication
2. Cache-stability as explicit governance policy
3. Tool-output asymmetry (audit vs. model context)
4. Hook-driven self-rehydration
5. Deterministic absorption shims for known artifact malformation
6. Turn-boundary nudge queue with rename-claim semantics

---

## A. Refinements to RGF-233 through RGF-241

### A.1 RGF-233 (Canonical Terminal Closeout Record) — extend the projection list

**Current scope:** packet, task-board, dossier, build-order, route-health, session status become projections of one terminal record.

**Refinement from research:** The OpenClaw-ACPX pattern is that ACP events are the only audit trail; everything else is a *machine-generated projection* over the events. The natural extension of RGF-233 is to also list:

- `closeout_provenance.md` (human-readable post-mortem) as a projection
- Any future operator-facing report as a projection

**Why it matters:** if any human-facing artifact remains *authored* (vs. *generated*), it will drift, and someone will edit the wrong copy. Pin the rule: anything a human reads at closeout is generated from the terminal record, never written by hand or by a model turn.

**Proposed brief addition:**
> 6. Document the principle: any future closeout-adjacent artifact must be a projection of the terminal record. Hand-authored or model-authored closeout prose is rejected at gov-check time.

### A.2 RGF-234 (Closeout Proof / Projection Sync Split) — name the cache-stability rule

**Current scope:** product proof check vs. projection sync are split phases; projection failure becomes typed debt, not generic closeout failure.

**Refinement from research:** Hermes' `AGENTS.md:521-535` codifies cache-stability as policy: no mid-conversation system-prompt mutation. RGF-234's projection sync is the natural place to also state the rule: **projection sync must not invalidate the cached prefix of any active role session**. Concretely: projection sync writes to disk; it does not push updates into a running session's context; it does not alter the system prompt of any open ACP session.

**Why it matters:** RGF-234 currently scopes "do not relaunch Integration Validator for projection drift". The cache-stability extension is "do not invalidate any role session's cached prompt for projection drift either". The cost saving is on every subsequent turn of every active role session, not just a single relaunch.

**Proposed brief addition:**
> 6. Projection sync writes only to durable storage (terminal record, packet, task board, dossier files). It does not signal active role sessions, does not modify their system prompts, and does not enqueue a steering message merely to inform them that projection state was updated. Any change a role session needs to know about must arrive as a typed action via the existing receipt/notification channels, not as a re-prompt.

### A.3 RGF-237 (Closeout Debt Report) — tighten the "compact" definition

**Current scope:** stable debt key, classification, owner surface, severity, blocks_product_outcome boolean, revalidation_required boolean, next mechanical command, reason.

**Refinement from research:** Pi's `details` vs. `content` split (`packages/agent/src/types.ts:291-302`) is the relevant pattern. The debt report has two readers:
1. The model session that needs to know "should I act?" (model-visible: 1-line summary + next command)
2. The human/audit reader that needs the full context (audit-only: severity, owner, reason, history)

These are different payloads with different lifecycles. RGF-237 currently bundles them into one report.

**Why it matters:** if the debt report is in any role session's context, every debt update is a cache miss on that session. If the report is split into a one-line model-visible summary and a full audit projection, the summary fits in a typed event (already covered by RGF-205 governed action envelope), and the full report stays in the terminal record / projection.

**Proposed brief addition:**
> 6. The debt report has two surfaces. The **model-visible surface** is one line per debt item (`<key> | <severity> | <next command>`). The **audit surface** carries the full classification, owner, reason, and history. Model-visible surface is what `orchestrator-next` returns in compact mode. Audit surface is read on demand by humans or by `phase-check CLOSEOUT --audit-mode`.

### A.4 RGF-238 (Closeout Repair Loop Breaker) — one explicit "absorb known malformations" pass

**Current scope:** hard budget (one auto pass + one manual remediation), record every attempt, escalation packet on non-convergence.

**Refinement from research:** Pi's `prepareArguments` shims (`tools/edit.ts:90-114`) and Hermes' `coerce_tool_args()` (`model_tools.py:382`) absorb known model misbehavior in code, not in workflow loops. A closeout repair loop that hits the same malformation 3 times is a workflow loop where a normalizer should be.

**Why it matters:** RGF-238 prevents loops; the absorption pattern prevents the *first occurrence* from ever needing repair. The repair-loop budget is the safety net; the absorption shim is the saving.

**Proposed brief addition:**
> 6. Before the first automated repair pass, run a "known-malformation absorber" stage: enumerate the top-N malformation modes observed historically (truncated trailing newline, missing dossier section, wrong heading level, smart-quote contamination, JSON-string-instead-of-array, CRLF/LF mismatch, etc.) and apply a deterministic normalizer for each. Only after absorption fails does the repair budget begin counting. Track absorber hit rates separately from repair attempts so the absorber set can grow with observation.

### A.5 RGF-241 (Closeout Breakpoint Scenario Harness) — add concurrent-writer fixtures

**Current scope:** broad scenario list (PASS/FAIL with various drift modes, missing verdict, signed-scope mismatch, etc.).

**Refinement from research:** OpenClaw-ACPX's queue-ownership pattern (`src/cli/queue/ipc.ts:89-101`) uses generation numbers to detect stale-owner races. The natural fixture is "two writers attempting terminal publication concurrently with stale and fresh generation numbers".

**Why it matters:** RGF-240 specifies monotonic state and atomic publication. RGF-241 should have a fixture that *exercises* RGF-240's stale-writer rejection path — not just `concurrent stale writer attempts downgrade` (which is in the list) but specifically `stale-generation writer + fresh-generation writer race`.

**Proposed brief addition:**
> 4. Add fixtures that exercise the RGF-240 generation/lock semantics directly: (a) two writers with the same generation racing on terminal record write; (b) stale-generation writer attempting to publish after a fresh-generation owner has settled; (c) recovery sequence where rescue Orchestrator inherits ownership from a crashed writer mid-publication.

---

## B. New RGF candidates from harness research

These are not in the current QUEUED tranche or in the board. Numbering is suggestive — actual IDs assigned at intake.

### B.1 RGF-242: Mid-Conversation Cache-Stability Policy and Ephemeral User-Message Injection

**Problem:** Active role sessions accumulate cache invalidations whenever governance state mutates (packet edits, dossier updates, MT receipt appends). Anthropic prefix caching is up to 75% input-token reduction; every cache miss multiplies the effective per-turn cost ~4x.

**Pattern source:** Hermes `AGENTS.md:521-535` and `agent/memory_manager.py:66-80` — cache-stability is a hard policy rule, and ephemeral context goes in the *user message* with a `<memory-context>` fence and disclaimer, not the system prompt.

**Required implementation:**
1. Codify a cache-stability rule in `Handshake_Codex_v1.4.md`: while a role session is active, its cached system prompt is immutable. Mutations land in storage; the next session sees them.
2. When the orchestrator must inject governance state into a running role session's turn, inject it into the *user message* with an explicit fence and a system-note disclaimer (e.g., `<governance-context source="orchestrator" trust="informational">...</governance-context>`).
3. Add a check that scans `wp-receipt-append`, `task-board-set`, dossier-sync, and projection writers for any path that signals an active role session beyond a typed event. Such paths fail closed under the new policy.
4. Document the opt-in `--now` flag for the rare case of forced invalidation, modeled after Hermes' slash-command discipline.

**Acceptance criteria:**
- Active role sessions experience zero system-prompt mutations between governance updates.
- A regression fixture proves a sequence of governance updates (packet edit, MT receipt, dossier append) does not invalidate the prefix cache of an active CODER session.
- All ephemeral injections go through a single `injectEphemeralUserContext` helper that emits the fenced format.

**Estimated impact:** Largest single token-cost reduction available pre-architecture-change. Order of magnitude: 30-50% input-token reduction on long WPs.

---

### B.2 RGF-243: Tool-Result Audit / Model Asymmetry (`details` vs. `content`)

**Problem:** Governance check outputs (phase-check, gov-check, packet-truth, validator-gate) currently return a single payload that is both human-readable and consumed by model sessions on subsequent turns. The full output enters every consumer's context, even when only a one-line verdict is needed for routing.

**Pattern source:** Pi `AgentToolResult` (`packages/agent/src/types.ts:291-302`). Every tool result has `content` (model-visible, kept minimal) and `details` (audit/UI/log only, never re-injected).

**Required implementation:**
1. Update governance-check libraries to return `{summary: string, details: object}` from every check. `summary` is a one-line model-visible result; `details` is a structured payload for logs and human review.
2. Wire `phase-check`, `gov-check`, `packet-truth-check`, `wp-communication-health-check`, `validator-gate-state` to write `details` to a structured log (e.g., `<wp_runtime>/check_details.jsonl`) and return only `summary` to the caller.
3. Compact mode (RGF-19, already DONE) becomes the default; verbose mode reads `details` from the log on demand.
4. Operator-viewport surfaces and dossier-sync read `details` from the log; model-visible surfaces read only `summary`.

**Acceptance criteria:**
- A phase-check run produces a one-line summary on stdout and a structured details record in the log; no consumer of stdout sees the full output by default.
- Dossier and operator-viewport surfaces continue to render the same level of detail by reading `details` from the log.
- A regression fixture proves a long-running orchestrator session does not accumulate phase-check verbose output in its context.

**Estimated impact:** 10-20% input-token reduction on orchestrator sessions; eliminates one of the three causes of the 110M-token orchestrator runs.

---

### B.3 RGF-244: Deterministic Artifact-Malformation Absorber

**Problem:** Models authoring WP packets, receipts, validator reports, and dossier entries produce a small set of recurring malformations (truncated trailing newlines, smart-quote substitution, JSON-as-string-vs-array, CRLF/LF mismatch, missing/extra heading levels, bullet-prefix on field lines, etc.). Each one currently triggers a repair turn or a workflow loop.

**Pattern source:** Pi `prepareArguments` shims (`tools/edit.ts:90-114`, `edit-diff.ts:34-55`) and Hermes `coerce_tool_args()` (`model_tools.py:382`). Known-deterministic model misbehavior is fixed in code before it reaches a workflow loop.

**Required implementation:**
1. Enumerate the top-N malformation modes observed historically. Sources: `validator-report-structure-check.mjs` violation patterns (the RGF-197 fixes are a starting catalog), repair-loop receipts, post-work-check failures, packet-truth check failures.
2. Implement one normalizer per mode under `roles_shared/scripts/lib/artifact-normalizers/`. Each normalizer is a pure function with a fixture-based test.
3. Wire the normalizer suite into:
   - `wp-receipt-append` (before persist)
   - `validator-report-structure-check` (before evaluation)
   - `packet-truth-check` (before evaluation)
   - The RGF-238 closeout repair pre-pass (per A.4 above)
4. Track per-normalizer hit counts in a structured log so the absorber set can grow from observed traffic.
5. Normalizers are *additive only*: they accept malformed input and emit canonical output. They never reject — that's the validator's job.

**Acceptance criteria:**
- A receipt with smart quotes, trailing whitespace, and `- field: value` bullet prefix is silently normalized at append time; the validator never sees the malformation.
- A regression suite covers each named malformation with a before/after fixture.
- The absorber-hit log shows non-zero hits within one WP cycle of deployment, proving real malformations are being absorbed.

**Estimated impact:** Eliminates the dominant remaining source of repair turns. RGF-197 already proved this pattern works on validator reports (39 violations → 0); generalize to all governance artifacts.

---

### B.4 RGF-245: Turn-Boundary Nudge Queue with Atomic Claim

**Problem:** Orchestrator and watchdog re-steering currently signals running role sessions in ways that can interrupt mid-tool-call or get lost during heavy host load. RGF-30 (event-driven happy-path relay) and RGF-166 (non-LLM relay watchdog) cover the wake-up semantics but not the on-disk delivery primitive.

**Pattern source:** Gastown `internal/nudge/queue.go` — JSON files in `<root>/.runtime/nudge_queue/<session>/`, FIFO with random suffix, atomic rename-claim, TTL (30 min normal, 2h urgent), requeue on failure, drained by `UserPromptSubmit` hook. CHANGELOG 1.0.1 (2026-04-25) documents Gastown's identical bug to ours.

**Required implementation:**
1. Add `roles_shared/scripts/session/nudge-queue-lib.mjs`:
   - Per-session queue dir at `<wp_runtime>/<wp_id>/nudges/<session_id>/`
   - Filename `<unix_nano>-<random_hex>.json` for FIFO with collision resistance
   - `enqueue(session_id, payload, {ttl, priority})` with `MaxQueueDepth=50`
   - `drain(session_id)` via rename-then-process (`*.json` → `*.claimed`)
   - Stale `.claimed` recovery: rename `*.claimed` older than 5 min back to `*.json` (not delete)
   - Requeue on failure preserves original timestamp
2. Wire CODER and WP_VALIDATOR session startup hooks to drain the queue on `UserPromptSubmit`-equivalent boundaries.
3. Migrate `orchestrator-steer-next` and `wp-relay-watchdog` to enqueue rather than direct stdin signaling for non-emergency wakes.
4. Direct stdin signaling remains available for emergency `RED_ALERT` cases (RGF-230) and explicit operator forces.

**Acceptance criteria:**
- A nudge enqueued during an active tool call is delivered at the next `UserPromptSubmit` without interrupting the call.
- Two concurrent drainers cannot double-deliver the same nudge (rename-claim race fixture).
- A drainer crash mid-claim does not lose the nudge (orphan-recovery fixture).
- Orchestrator session token cost drops measurably (no more re-steering during cache-warm windows).

**Estimated impact:** Solves the polling-waste / mid-stream-interruption complaint. CHANGELOG 1.0.1 of Gastown is direct evidence the pattern fixes the same context-bloat-from-accumulating-mail bug.

---

### B.5 RGF-246: Hook-Driven Session Self-Rehydration

**Problem:** The orchestrator currently constructs the prompt for spawned role sessions, including governance state, MT identity, work assignment, and authority surface. This makes the orchestrator (a) a bottleneck on every relaunch, (b) responsible for context completeness, and (c) the source of "I forgot to include X" failure modes.

**Pattern source:** Gastown `gt prime` runs in the agent's own `SessionStart` hook (`internal/hooks/templates/claude/settings-autonomous.json:100-110`). The agent self-rehydrates from beads at session start; the orchestrator only dispatches a tiny directive (which WP, which role, which MT).

**Required implementation:**
1. Move governance-state assembly out of `launch-cli-session.mjs` into a per-role startup hook (e.g., `roles_shared/scripts/session/role-self-prime.mjs`).
2. Orchestrator dispatch becomes: drop a directive `{wp_id, mt_id, role}` into the queue (or set as launch arg); spawn the session with the hook configured; the hook reads canonical truth (terminal record from RGF-233, packet projection, MT board, memory) and assembles the prompt.
3. Reuse-on-resume: a session resumed from compaction or restart re-runs `role-self-prime` and re-reads canonical truth, rather than re-reading the orchestrator's last prompt.
4. The orchestrator stops constructing prompts for governed roles (it can still construct prompts for itself or for the rescue path).

**Acceptance criteria:**
- A CODER session spawned via the new hook builds its own prompt from canonical truth and runs without the orchestrator providing context.
- A CODER session resumed after compaction self-rehydrates without orchestrator intervention.
- Orchestrator session turn count on a normal WP drops because prompt construction is no longer an orchestrator turn.

**Estimated impact:** ~30% reduction in orchestrator turn count on managed WPs. Compounds with RGF-242 because each freed orchestrator turn no longer invalidates anything.

---

### B.6 RGF-247: Validator-as-Tool-Result for Mechanical Per-MT Verdicts

**Problem:** Per-MT validation currently routes through ACP to a separate WP_VALIDATOR session. Round-trip cost: orchestrator turn → ACP launch → validator reads packet + receipts + diff → emits verdict → orchestrator reads verdict → orchestrator updates dossier → next turn. Each "→" is a model call.

**Pattern source:** Pi's `afterToolCall` hook + `completeSimple()` inline (`packages/agent/src/types.ts:75-101`, `compaction.ts:574-578`). Validators that do mechanical checks run as a synchronous tool call inside the coder session, returning a typed verdict object. Pi's compaction calls the same provider for summarization inline; same pattern.

**Required implementation:**
1. Identify the per-MT validation surface that is *mechanical* (boundary check, scope containment, file-list match against MT contract). Call this the "mechanical track" (consistent with RGF-79's dual-track validator model).
2. Implement mechanical-track validation as a synchronous helper invoked from the coder session as a tool call: `wp-validator-mechanical-review --mt MT-N`. Returns typed `{verdict: PASS|FAIL, concerns: [...], evidence: {...}}`.
3. The result is appended as a typed receipt; the routing layer reads the receipt and either continues to next MT or routes back to coder remediation.
4. The *judgment track* (Master Spec primitive retention, code review, anti-vibe) remains as a separate ACP session — that's where actual model judgment is needed.
5. For the mechanical track, the coder session does not wait on a separate validator session; the verdict lands inside the coder turn.

**Acceptance criteria:**
- Per-MT mechanical verdicts come back without spawning a WP_VALIDATOR ACP session.
- The dual-track model (RGF-79, RGF-191) is preserved: PASS still requires both tracks.
- A regression fixture proves mechanical-track FAIL on a clear scope violation routes the coder session into immediate remediation without orchestrator mediation.
- WP run-time on a clean WP drops because per-MT validation no longer adds an ACP round-trip.

**Estimated impact:** Removes the dominant source of per-MT round-trip cost. Compounds well with B.5 (hook self-rehydration) and B.4 (nudge queue).

---

### B.7 RGF-248: Named-Verb Inter-Role Message Schema (Beyond Closeout)

**Problem:** Per-MT and per-WP communication still flows through prose receipts that the model authors and that other roles read. Closeout canonicalization (RGF-233) addresses terminal events; per-MT events remain prose.

**Pattern source:** Gastown `docs/design/mail-protocol.md` — named-verb mails (POLECAT_DONE, MERGE_READY, REWORK_REQUEST) with fixed Subject and 3-5 labelled body lines. The handoff is typed; the schema is small enough that malformation is rare.

**Required implementation:**
1. Define ~6-8 named verbs covering all routine inter-role traffic:
   - `MT_HANDOFF` (coder → wp_validator)
   - `MT_VERDICT` (wp_validator → coder/orchestrator)
   - `MT_REMEDIATION_REQUIRED` (wp_validator → coder)
   - `WP_HANDOFF` (coder → orchestrator)
   - `INTEGRATION_VERDICT` (integration_validator → orchestrator)
   - `CONCERN` (any role → orchestrator)
   - `PHASE_TRANSITION` (orchestrator → all)
   - `RELAUNCH_REQUEST` (orchestrator → role)
2. Each verb has a fixed body schema (3-5 labelled lines max) defined in `roles_shared/schemas/inter_role_verbs/`.
3. `wp-receipt-append` accepts `--verb <NAME>` and validates against the schema. Non-verb receipts continue to work (compatibility) but get a deprecation warning.
4. Routing helpers (RGF-200 receipt-driven route convergence) read verb-typed receipts in priority order; prose receipts route through a fallback parser.
5. Operator-facing dossier projection materializes verb receipts into prose for human reading; the model never reads the prose.

**Acceptance criteria:**
- A WP run can complete using only verb-typed receipts.
- Receipt-driven routing reads verb fields directly without parsing prose.
- A regression suite covers schema validation for every defined verb.
- Dossier projection is identical regardless of whether receipts were verb-typed or legacy prose.

**Estimated impact:** Removes the dominant source of artifact malformation at the per-MT level. Compounds with RGF-244 (absorber) — verbs are small enough that absorption rarely fires.

---

### B.8 RGF-249: Predecessor-Session Lookup for Compaction / Restart

**Problem:** When a session compacts or restarts, it re-reads WP/MT documents and packet history to reconstruct context. This is expensive and partially redundant with the memory injection path.

**Pattern source:** Gastown `gt seance` — agents query their predecessor's `.events.jsonl` to recover context without re-reading the codebase.

**Required implementation:**
1. Every governed session writes a structured `events.jsonl` with key transitions (tool calls, receipts emitted, files touched, MT progression).
2. On session start (post-compaction or post-restart), the self-prime hook (RGF-246) queries the predecessor session's `events.jsonl` for the same `(wp_id, role)` pair and includes a compact summary in the new session's initial user message (using the `<governance-context>` fence from RGF-242).
3. The events log is small (~1KB per turn for structured events) and the summary is bounded (~500 tokens for the predecessor summary).

**Acceptance criteria:**
- A compacted CODER session resumes without re-reading the WP packet, validator reports, or MT board on first turn.
- The predecessor summary fits within a 500-token budget for normal WP lengths.
- A regression fixture proves the resumed session understands its current state from the predecessor summary alone.

**Estimated impact:** Reduces re-entry cost after compaction or restart. Particularly relevant for long WPs that hit context limits.

---

## C. DONE items worth re-reviewing against research

These items are marked DONE on the board. Quick check whether they fully realize the research-pattern they intended.

### C.1 RGF-101 (SQLite Communication Backbone) vs. Hermes SessionDB

RGF-101 implements WP_COMMUNICATIONS as SQLite. Hermes' analog is `~/.hermes/sessions.db` (SQLite + FTS5). Worth checking:
- Does Handshake's SQLite have FTS5 enabled for receipt content search?
- Is there a `session_search` equivalent helper that role sessions can call to recall prior receipts on demand (vs. reading them at startup)?

If not: a small extension would let roles query receipts via tool call instead of injecting them into the prompt.

### C.2 RGF-150 (Single-Writer Lifecycle Truth) vs. ACPX queue ownership

RGF-150 made receipt-driven review reconciliation the single writer for review-stage lifecycle. Worth checking against ACPX's `assertOwnerGeneration` (`src/cli/queue/ipc.ts:89-101`):
- Does the writer use generation-numbered ownership for stale-owner detection?
- Are stale-owner attempts rejected with a typed error rather than failing silently?

If not: B.4 (nudge queue) would benefit from inheriting the same generation pattern, and C.2 could be a small follow-up to add it to the existing single-writer.

### C.3 RGF-189 (Mechanical Orchestrator Governance) — full reach

RGF-189 codified that orchestrator runs governance checks via direct just/node calls, not ACP. Worth checking:
- Are there any remaining orchestrator paths that route mechanical checks through ACP (e.g., closeout-truth-sync)?
- Does the orchestrator protocol fully enforce "no ACP for mechanical work" or are there exceptions still in the wild?

If exceptions exist: a small follow-on would close them. The Pi philosophy ("don't make a protocol where a function call works") suggests no exceptions.

### C.4 RGF-205 (Governed Action Envelope) vs. ACPX typed messages

RGF-205 is the closest thing on the board to the ACPX `QueueSubmitRequest`/`QueueCancelRequest`/`QueueSetModeRequest` typed-message pattern. Worth confirming:
- Are all inter-role transitions covered by typed envelopes, or does prose still flow between roles for some traffic?
- B.7 (named verbs) extends this if not.

---

## D. Suggested sequencing

The 233-241 tranche should ship as planned. The harness-research additions interleave naturally:

**Tier 1 (fast wins, parallel to or after 233-241):**
- B.1 (RGF-242: cache-stability policy) — week of work, big input-token reduction
- B.2 (RGF-243: tool-result asymmetry) — week of work, compounds with B.1
- B.3 (RGF-244: artifact normalizer suite) — 1-2 weeks, eliminates dominant repair source

**Tier 2 (architectural, after 233-241):**
- B.4 (RGF-245: nudge queue) — 2 weeks, solves polling waste
- B.5 (RGF-246: hook self-rehydration) — 2-3 weeks, requires per-role hook templates
- B.7 (RGF-248: named verbs) — 2-3 weeks, compounds with B.3 absorber

**Tier 3 (deeper, after Tier 2 lands):**
- B.6 (RGF-247: validator-as-tool-result) — 3-4 weeks, requires Tier 2 in place
- B.8 (RGF-249: predecessor session lookup) — 1 week, depends on B.5

**Refinements (one-line additions to existing briefs):**
- A.1 through A.5 are low-effort additions to the 233-241 briefs and can land in the same tranche.

---

## E. What changes for the model, what stays for the operator

The proposals reduce **model-context cost** without changing what the operator sees or relies on. Specifically:

- **WP packets, microtask contracts, dossiers, validator reports, post-mortems** — these are the operator's window into work the operator cannot read in code. They stay. B.1 (cache-stability) and B.2 (`details`/`content`) make sure the packet is *not* what model sessions read on every turn; B.7 (named verbs) makes the inter-role traffic typed so the *packet projection* stays human-readable while the wire format is small. The packet's role as the human-facing source of truth is unchanged.
- **The role split** — CODER / WP_VALIDATOR / INTEGRATION_VALIDATOR / ORCHESTRATOR remain distinct roles with distinct authority. The role split is what prevents self-checking and gives the operator independent verification on work the operator cannot read. B.6 only changes the *transport* of the mechanical track of WP_VALIDATOR (it becomes a synchronous helper call) — the judgment track and INTEGRATION_VALIDATOR remain separate ACP sessions because that's where independent model judgment is the entire point.
- **ACP** stays as the transport for any work where a model is making a judgment. RGF-189 already moved mechanical governance off ACP; the proposals layer typed events on top of ACP, not replace it.
- **The memory system** (RGF-115 through RGF-147) is largely consistent with the harness research and is not proposed for rewrite. Section C.1 suggests a small additive extension (FTS5 `session_search` helper) so role sessions can recall on demand instead of injecting on startup.

The headline: nothing the operator reads or governs gets simplified away. What gets simplified is the *amount of governance text that enters a model's prompt*. The packet remains the spec; the dossier remains the audit; the role boundaries remain enforced. The model just stops paying tokens to re-read its own bookkeeping every turn.

---

## F. Source pointers

- Synthesis document: `.GOV/reference/research_and_papers/harnesses/00_HARNESS_COMPARATIVE_ANALYSIS.md`
- Per-harness drafts: `01_pi.md`, `02_hermes.md`, `03_openclaw.md`, `04_gastown.md`
- Repo comparators: cloned harness repos `{pi-mono, hermes-agent, openclaw, openclaw-acpx, gastown}` in the operator's harnesses workspace
- Companion brief: `REPO_GOVERNANCE_REFACTOR_IMPLEMENTATION_BRIEFS_20260426.md` (the 233-241 tranche)

Specific code citations used in this addendum:
- Pi: `packages/agent/src/types.ts:291-302` (details/content), `tools/edit.ts:90-114` (prepareArguments), `agent/src/agent.ts:113-144` (PendingMessageQueue)
- Hermes: `AGENTS.md:521-535` (cache stability), `model_tools.py:382` (coerce_tool_args), `agent/memory_manager.py:66-80` (memory-context fence)
- OpenClaw-ACPX: `src/cli/queue/ipc.ts:89-101` (queue ownership), `src/cli/queue/messages.ts:24-120` (typed messages), `docs/2026-02-25-warm-session-owner-architecture.md` (detached owner)
- Gastown: `internal/nudge/queue.go` (nudge queue), `docs/design/mail-protocol.md` (named verbs), `internal/hooks/templates/claude/settings-autonomous.json:100-110` (gt prime hook), CHANGELOG 1.0.1 (identical bug fix evidence)
