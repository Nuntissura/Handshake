# MEMORY_MANAGER_PROTOCOL

## Role Definition

The Memory Manager has two modes: a mechanical pre-pass (script) and an intelligent review (model session). The script handles deterministic maintenance cheaply. The model session handles judgment-based work that requires understanding content.

**Authority:** Memory DB read/write only. No product code, no protocol edits, no codex changes, no WP management.

**Worktree:** `wt-gov-kernel` on branch `gov_kernel`.

### Mechanical Pre-Pass (`just launch-memory-manager`)

A Node.js script that runs deterministically, no tokens consumed. It is intentionally conservative: extraction, soft decay, embedding refresh, recall effectiveness audit, stats collection, and report-first candidate detection for stale / contradictory / old low-value entries. It must avoid destructive judgment calls during automatic startup/closeout runs. Outputs `MEMORY_HYGIENE_REPORT.md`. Runs automatically at orchestrator startup (staleness-gated: >24h AND >10 new entries).

### Intelligent Review Session (`just launch-memory-manager-session`)

A model session (default profile: `OPENAI_GPT_5_5_XHIGH`) launched after the mechanical pre-pass. The model reads the hygiene report, queries the DB, and performs work the script cannot:

- **Quality assessment** — read procedural fix patterns and judge if they are still correct against current code
- **Contradiction resolution** — read both conflicting entries, understand context, decide which is right
- **Stale entry analysis** — determine if a memory with gone file references still has general applicability
- **RGF candidate drafting** — write real evidence-based governance improvement proposals
- **Conversation insight review** — find insights the FTS similarity missed, promote manually
- **Operator-reported entry audit** — verify high-value entries are still accurate and well-worded

The model appends an `## Intelligent Review` section to the report, writes proposals to `.GOV/roles/memory_manager/proposals/`, records `just repomem close ...`, and then stops after the governed turn completes.

**Preferred profile:** `OPENAI_GPT_5_5_XHIGH` with `model_reasoning_effort=xhigh`. Claude profiles remain available by explicit override when the quality/rate-limit tradeoff is acceptable.

**ACP integration:** The intelligent session is launched headless through ACP as a governed role (`MEMORY_MANAGER`) with a synthetic WP-ID (`WP-MEMORY-HYGIENE_<timestamp>`). It must not open or focus a visible terminal on the default path. This gives it:
- Structured communication via receipts (`MEMORY_PROPOSAL`, `MEMORY_FLAG`, `MEMORY_RGF_CANDIDATE`)
- Orchestrator visibility via `check-notifications`
- Session registry tracking (launch, steer, close)
- Proposals backed up to `.GOV/roles/memory_manager/proposals/<topic>_<timestamp>.md`

Because this lane is packetless, the synthetic communication files under `gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-MEMORY-HYGIENE_<timestamp>/` become the authoritative receipt and notification surface for Memory Manager findings.
Clarification: governed completion is evidenced by the `SESSION_COMPLETION` notification after the Memory Manager stops its turn. Explicit ACP `CLOSE_SESSION` remains orchestrator-owned when the steerable thread itself should be retired.

**Lifecycle:** `just launch-memory-manager-session` → mechanical pre-pass → ACP session start → `memory-manager-startup` → repomem open → review work → write proposals (receipts + backup files) → repomem close → stop after the governed turn settles and `SESSION_COMPLETION` is emitted. Explicit ACP `CLOSE_SESSION` remains orchestrator-owned. MUST NOT leave orphan terminals.

## Governance Surface Reduction Discipline

- Memory hygiene should remain centered on the existing `just memory-*` command family plus one primary output artifact: `MEMORY_HYGIENE_REPORT.md`.
- Do not normalize extra public memory-maintenance wrappers, duplicate reports, or side-channel command surfaces when the existing memory command family and report can absorb the work.
- For scripts and recipes specifically, prefer expanding the canonical `memory-*` surfaces rather than adding sibling public scripts that would normally run in the same hygiene pass.
- When multiple deterministic hygiene checks or repairs belong to the same memory-maintenance pass, fold them into the existing `memory-*` bundle and `MEMORY_HYGIENE_REPORT.md` instead of exposing more leaf commands.
- Keep a separate public memory script only when owner boundary, side-effect class, runtime/topology assumptions, primary debug artifact, or operator usefulness materially differs.
- If a new live memory-governance surface is genuinely required, record why the existing surface is insufficient, who owns the new surface, and what the primary debug artifact is.

## When It Runs

| Trigger | Mode | Who launches | Condition |
|---|---|---|---|
| Orchestrator startup | Mechanical | `just orchestrator-startup` | Staleness gate: >24h since last run AND >10 new entries |
| Integration Validator closeout | Mechanical | `just phase-check CLOSEOUT WP-{ID}` | Always before WP merge to main |
| Operator manual (mechanical) | Mechanical | `just launch-memory-manager [--force]` | On demand |
| Operator manual (intelligent) | ACP session | `just launch-memory-manager-session [host] [model]` | On demand; runs mechanical pre-pass first, then launches governed ACP session |
| Gov-flush | Mechanical | `just gov-flush` | Step 6: memory hygiene before NAS backup |

## Memory System Architecture (What You Manage)

The memory DB has 3 memory types, 1 snapshot type, 1 conversation log, and 6 tables:

**Types stored:**
- `procedural` — fix patterns, error-fix pairs, smoketest findings (coder-facing fail log)
- `semantic` — distilled facts, architecture patterns, positive controls (coder+validator context)
- `episodic` — timestamped session events, receipt extractions, pre-task snapshots (orchestrator history)

**Conversation log (`conversation_log` table):**
- Cross-session conversational memory — captures what was discussed, decided, and discovered
- Checkpoint types (10): `SESSION_OPEN`, `PRE_TASK`, `INSIGHT`, `DECISION`, `ERROR`, `ABANDON`, `CONCERN`, `ESCALATION`, `RESEARCH_CLOSE`, `SESSION_CLOSE`
- Written by WP-bound roles via `just repomem` commands; injected into startup prompts as `CONVERSATION CONTEXT`
- Memory Manager is the packetless hygiene exception: it still opens/closes its own repomem session, but normal WP repomem coverage gates exclude it and use `MEMORY_*` receipts plus backup proposal files as its durable evidence.
- `INSIGHT` entries are the highest-signal source of institutional knowledge — operator decisions, corrections, and discoveries
- `DECISION`/`ERROR`/`ABANDON`/`CONCERN`/`ESCALATION` provide granular WP diagnostic context — these land in the workflow dossier (EXECUTION or CONCERNS section) via `inject-repomem`
- Quality-gated: >=80 chars for open/close/insight/decision/abandon/concern, >=40 for pre-task/error/escalation; close requires `--decisions`

**Snapshot types (stored as episodic with `snapshot_type` column):**
- `PRE_WP_DELEGATION`, `PRE_STEERING`, `PRE_RELAY_DISPATCH`, `PRE_PACKET_CREATE`, `PRE_CLOSEOUT`, `PRE_BOARD_STATUS_CHANGE` — mechanical, captured automatically at script entry points (importance 0.85)
- `INTENT` — judgment-based, captured by roles before complex reasoning (importance 0.9)

**Write paths you should know about:**
- Receipt extraction (mechanical, trust 1.0) — every high-signal receipt auto-creates a memory
- Smoketest extraction (mechanical, trust 0.9) — SMOKE-FIND/SMOKE-CONTROL → procedural/semantic
- Check failure capture (mechanical, trust 0.8) — validator-scan, pre/post-work failures
- Mid-session capture (judgment, trust 0.7) — roles calling `just memory-capture`
- Session-end flush (mechanical, trust 0.5) — CLOSE_SESSION writes session summary
- Pre-task snapshots (mechanical, trust 0.85-0.9) — scripts and roles capture context before complex ops

**Write-time safeguards (already applied before you see the data):**
- Novelty scoring: FTS5 near-duplicate → 0.3x importance [RGF-135]
- Supersession: new procedural on same file_scope marks old one consolidated [RGF-137]
- Contradiction detection: same file_scope + different topic → both flagged to 0.3 [RGF-141]
- Date normalization: relative dates → absolute at write time [RGF-143]
- Dedup: exact topic+wp+type match → skip at write time

**Read paths (who consumes what you curate):**
- Coder sessions: procedural only (FAIL LOG), 1500 tokens, WP-scoped + file-scope boosted [RGF-120/124/128]
- Validator sessions: procedural + semantic (FAIL LOG + CONTEXT), 1500 tokens, WP-scoped [RGF-120/124]
- Orchestrator sessions: all types (GOVERNANCE MEMORY), 2000 tokens, cross-WP, systemic boost [RGF-125]
- All sessions: up to 3 recent SNAPSHOTS (mechanical + intent), most recent per type [RGF-147]

**Scoring formula at injection time:**
`importance × recency_decay × access_boost × file_scope_match × staleness_factor × trust_multiplier × session_diversity_cap`

Your job is to ensure the data that flows through this formula is clean, relevant, and well-calibrated.

## What It Does (Per Run)

### Phase 1: Health Assessment (read-only)

1. **Stats check** — report active/consolidated counts, type distribution, last compaction, DB size vs 500-entry cap
2. **Trust distribution** — query `source_artifact` counts to check if low-trust sources are accumulating disproportionately
3. **Snapshot compliance** — check INTENT + mechanical snapshot counts over last 7 days (signals protocol compliance by roles)
4. **Embedding coverage** — count entries in `memory_embeddings` vs `memory_index` active entries; report gap percentage
5. **Conversation log stats** — total entries, distinct sessions, insight count

### Phase 2: Active Maintenance

6. **Extract** — run receipt + smoketest extraction (idempotent)
7. **Compact** — run Ebbinghaus decay + budget pruning (`decayRate=0.1`, `pruneThreshold=0.05`)
8. **Stale file_scope audit** — flag procedural/semantic entries >7d old where all referenced files are gone AND access_count < 2
9. **Contradiction candidate audit** — find entries with `metadata.contradiction=true`, group by `file_scope`, and report candidate pairs for intelligent review. Do not mechanically pick a winner.
10. **Supersession chain audit** — check `metadata.superseded_by` chains; if the successor was itself pruned/consolidated, un-consolidate the original (restore importance 0.4)
11. **Conversation insight promotion** — FTS5 keyword similarity across `conversation_log` INSIGHT and DECISION entries; promote to semantic memory (importance 0.8) when the same insight appears across 3+ sessions. Also promote decisions from `decisions` column that repeat across 2+ sessions. CONCERN entries recurring across 2+ WPs should be promoted as semantic (systemic risk patterns).
12. **Conversation log pruning** — delete sessions >30 days old with no INSIGHT, DECISION, ERROR, CONCERN, or RESEARCH_CLOSE checkpoints (sessions with only OPEN/CLOSE/PRE_TASK have low diagnostic value)
13. **Age-based consolidation candidates** — report entries >30 days old with access_count < 2 AND importance < 0.4 for intelligent review. Do not mechanically consolidate them during automatic runs.
14. **Embedding refresh** — if embedding coverage <50% and Ollama is available, run `just memory-embed --batch 20`
15. **Recall effectiveness audit** — find `operator-reported` and `memory-capture` source entries that have decayed below importance 0.5; restore to 0.8 (these are high-value fail captures that must not decay). Report total active operator-reported count

### Phase 3: Pattern Analysis

16. **Novelty calibration** — report % of recent entries hitting low importance (threshold: 30%)
17. **Session diversity** — flag sessions contributing >5 memories to the active pool
18. **Intent snapshot compliance** — report intent count over 7 days (concern if 0 or >20)
19. **Conversation checkpoint compliance** — report all 10 checkpoint type counts (OPEN/CLOSE/INSIGHT/DECISION/ERROR/ABANDON/CONCERN/ESCALATION/PRE_TASK/RESEARCH_CLOSE) for 7 days; flag unclosed sessions, zero insights, or zero decisions (roles should be recording choices), and surface recent per-WP repomem coverage debt when materially active roles lack OPEN/CLOSE/WP-durable proof
20. **RGF candidate drafting** — cross-WP procedural patterns (3+ WPs) + high-access memories (10+ accesses) → draft governance improvement candidates

### Phase 4: Report

21. **Write report** — `gov_runtime/roles_shared/MEMORY_HYGIENE_REPORT.md` (see Output Format below)

## What It Does NOT Do

- Edit protocols, codex, AGENTS.md, or any `.GOV/` documentation
- Touch product code (`src/`, `app/`, `tests/`)
- Directly modify the governance task board (drafts candidates only)
- Launch or manage other sessions
- Make workflow decisions
- Stay running after its tasks are complete

## Available Commands

```
# Read-only
just memory-stats
just memory-search "<query>" [--type T] [--wp WP-{ID}]
just memory-prime <WP-{ID}> [--budget N]
just memory-recall <ACTION> [--wp WP-{ID}] [--budget N]
just memory-patterns [--min-wps N] [--min-access N]
just memory-compact --dry-run
just memory-debug-snapshot [WP-{ID}|INTENT]
just memory-recall <ACTION> [--wp WP-{ID}]

# Write
just memory-compact [--older-than 30d]
just memory-refresh --force-compact
just memory-flag <id> "<reason>"
just memory-capture <type> "<insight>" [--wp WP-{ID}] [--scope "files"]
just memory-embed [--batch N]
just memory-manager-proposal <WP-{ID}> <actor-session> "<summary>" "<backup_ref>" [correlation_id]
just memory-manager-flag-receipt <WP-{ID}> <actor-session> "<summary>" "<backup_ref>" [correlation_id]
just memory-manager-rgf-candidate <WP-{ID}> <actor-session> "<summary>" "<backup_ref>" [correlation_id]
```

## Output Format

Write `gov_runtime/roles_shared/MEMORY_HYGIENE_REPORT.md`:

```markdown
# Memory Hygiene Report
- Run: <timestamp>
- Mode: mechanical (no model session)
- Duration: <seconds>
- Trigger: <forced | staleness gate passed>

## Health
- Active: <N> | Consolidated: <N> | By type: procedural=<N> semantic=<N> episodic=<N>
- Snapshots (7d): mechanical=<N> intent=<N>
- Last compaction: <timestamp>
- DB size status: <OK | APPROACHING_CAP | OVER_CAP> (<N>/500)
- Embedding coverage: <N>/<N> (<percent>%)
- Trust distribution: <source>=<N> ...
- Conversation log: <N> entries, <N> sessions, <N> insights

## Actions Taken
- Extracted: receipts=<N>, smoketests=<N>
- Compacted: processed=<N>, decayed=<N>, pruned=<N>
- Stale candidates: <N> (report-only)
- Contradiction candidates: <N> (report-only)
- Supersession chains repaired: <N>
- Conversation insights promoted: <N>
- Conversation entries pruned: <N>
- Age-consolidation candidates: <N> (report-only)
- Recall audit: restored <N> operator-reported entries from decay
- Embeddings added: <N>

## Contradiction Candidates (for intelligent review)
- #<id> vs #<id>: <why human/model review is still required>

## Maintenance Candidates (for intelligent review)
- Stale file-scope candidates: <N>
- Age-consolidation candidates: <N>

## Recall Effectiveness
- Operator-reported entries: <N> active (<N> restored from decay)

## Calibration Notes
- Novelty penalty rate: <percent> of recent entries hit low importance (threshold: 30%)
- Session diversity: <sessions with >5 memories in active pool>
- Intent snapshot compliance: <count in last 7d> (<assessment>)
- Conversation checkpoints (7d): OPEN=<N> CLOSE=<N> INSIGHT=<N> PRE_TASK=<N> RESEARCH_CLOSE=<N>
- WP repomem coverage debt (7d): <N> recent WPs in debt

## WP Repomem Coverage Debt
- <WP-{ID}>: active_roles=<roles> | debt_keys=<ROLE:DEBT_KEY,...>

## RGF Candidates (for orchestrator review)
- CANDIDATE: <title> — <evidence summary>

## Recommendations
- <free-form observations for orchestrator>

## Post-Run Stats
- Active: <N> | Consolidated: <N>
- By type: procedural=<N> semantic=<N> episodic=<N>
```

## Rubric

See `.GOV/roles/memory_manager/docs/MEMORY_HYGIENE_RUBRIC.md` for scoring criteria when deciding what to flag, prune, collapse, or promote.

## Canonical Reference

`.GOV/roles/memory_manager/MEMORY_MANAGER_PROTOCOL.md` is the operational guide for the memory system this role manages; `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md` is the command syntax reference.
