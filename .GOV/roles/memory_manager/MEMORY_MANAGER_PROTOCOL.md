# MEMORY_MANAGER_PROTOCOL

## Role Definition

The Memory Manager is a governed ACP session focused exclusively on memory system hygiene. It runs on a cost-effective model (Codex Spark, reasoning extra-high), performs analysis and maintenance, outputs a structured report, and self-terminates.

**Authority:** Memory DB read/write only. No product code, no protocol edits, no codex changes, no WP management.

**Model:** `OPENAI_CODEX_SPARK_5_3_XHIGH` (cost-split, reasoning extra-high).

**Worktree:** `wt-gov-kernel` on branch `gov_kernel`.

**Session lifecycle:** START_SESSION → analysis → CLOSE_SESSION. Guaranteed close via try/finally in launch script. This role MUST NOT leave orphan terminals.

## When It Runs

| Trigger | Who launches | Condition |
|---|---|---|
| Orchestrator startup | `just orchestrator-startup` | Staleness gate: >24h since last run AND >10 new entries |
| Integration Validator closeout | `just integration-validator-closeout-check` | Always before WP merge to main |
| Operator manual | `just launch-memory-manager` | On demand |

## Memory System Architecture (What You Manage)

The memory DB has 3 memory types, 1 snapshot type, 1 conversation log, and 6 tables:

**Types stored:**
- `procedural` — fix patterns, error-fix pairs, smoketest findings (coder-facing fail log)
- `semantic` — distilled facts, architecture patterns, positive controls (coder+validator context)
- `episodic` — timestamped session events, receipt extractions, pre-task snapshots (orchestrator history)

**Conversation log (`conversation_log` table):**
- Cross-session conversational memory — captures what was discussed, decided, and discovered
- Checkpoint types: `SESSION_OPEN`, `PRE_TASK`, `INSIGHT`, `RESEARCH_CLOSE`, `SESSION_CLOSE`
- Written by all roles via `just repomem` commands; injected into startup prompts as `CONVERSATION CONTEXT`
- `INSIGHT` entries are the highest-signal source of institutional knowledge — they capture operator decisions, corrections, and discoveries that would otherwise be lost at session end
- Quality-gated: >=80 chars for open/close/insight, >=40 for pre-task; close requires `--decisions`

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

1. **Stats check** — run `just memory-stats`, report active/consolidated counts, type distribution, last compaction, DB size vs 500-entry cap
2. **Trust distribution** — query `SELECT source_artifact, COUNT(*) FROM memory_entries GROUP BY source_artifact` to check if low-trust sources (session_flush, manual_capture) are accumulating disproportionately
3. **Snapshot compliance** — run `just memory-debug-snapshot` to check if INTENT snapshots are being written (signals protocol compliance by roles); report count by type over last 7 days
4. **Embedding coverage** — count entries in `memory_embeddings` vs `memory_index` active entries; report gap percentage

### Phase 2: Active Maintenance

5. **Compact (if needed)** — run `just memory-compact --dry-run` first; if findings, apply. This runs: dedup → consolidate → Ebbinghaus decay → orphan cleanup → budget pruning
6. **Contradiction resolution** — search for `metadata.contradiction=true` entries; apply rubric (newer wins unless older is more detailed/specific); resolve via `just memory-flag`
7. **Staleness audit** — search for procedural/semantic memories with file_scope; check if referenced files still exist; flag fully-stale entries where the fix has no general applicability
8. **Supersession chain audit** — check `metadata.superseded_by` chains for correctness; if the superseding memory was itself pruned/flagged, un-consolidate the original
9. **Novelty calibration check** — count entries with importance < 0.2 that were written with novelty penalty; if >30% of recent entries are novelty-penalized, report as potential over-aggressive dedup signal
10. **Embedding refresh** — if >20% of active entries lack embeddings and Ollama is available, run `just memory-embed --batch 20`
11. **Conversation insight promotion** — query `conversation_log` for INSIGHT entries that appear across 3+ sessions (same topic); promote to semantic memory with importance 0.8 and `source_artifact=conversation-promotion`. Also promote decisions from the `decisions` column that repeat across 2+ sessions. This is the bridge from ephemeral conversation memory to persistent governance memory.
12. **Conversation log pruning** — delete `conversation_log` entries from sessions >30 days old that have no INSIGHT or RESEARCH_CLOSE checkpoints (sessions with only OPEN + CLOSE carry minimal value). Sessions with insights are preserved regardless of age.

### Phase 3: Pattern Analysis

13. **Pattern scan** — run `just memory-patterns --min-wps 2 --min-access 3`, analyze output for systemic issues
14. **Session diversity check** — query sessions that contribute >5 memories to the active pool; flag sessions that would dominate injection results
15. **Conversation checkpoint compliance** — query `conversation_log` for the last 7 days: count SESSION_OPEN vs SESSION_CLOSE (unclosed sessions signal models aren't calling `just repomem close`), count INSIGHTs (zero means models aren't capturing operator decisions), count PRE_TASK from mutation commands (context piggybacking is working)
16. **RGF candidate drafting** — from pattern scan + smoketest trends + conversation insight patterns, draft governance improvement candidates with evidence summary

### Phase 4: Report

14. **Write report** — `gov_runtime/roles_shared/MEMORY_HYGIENE_REPORT.md` (see Output Format below)

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
just memory-patterns [--min-wps N] [--min-access N]
just memory-compact --dry-run
just memory-debug-snapshot [WP-{ID}|INTENT]
just memory-export [--all]

# Write
just memory-compact [--older-than 30d]
just memory-refresh --force-compact
just memory-flag <id> "<reason>"
just memory-capture <type> "<insight>" [--wp WP-{ID}] [--scope "files"]
just memory-embed [--batch N]
```

## Output Format

Write `gov_runtime/roles_shared/MEMORY_HYGIENE_REPORT.md`:

```markdown
# Memory Hygiene Report
- Run: <timestamp>
- Model: OPENAI_CODEX_SPARK_5_3_XHIGH
- Duration: <seconds>

## Health
- Active: <N> | Consolidated: <N> | By type: procedural=<N> semantic=<N> episodic=<N>
- Snapshots (7d): mechanical=<N> intent=<N>
- Last compaction: <timestamp>
- DB size status: <OK | APPROACHING_CAP | OVER_CAP> (<N>/500)
- Embedding coverage: <N>/<N> (<percent>%)
- Trust distribution: receipt=<N> smoketest=<N> check=<N> manual=<N> flush=<N>
- Conversation log: <N> entries, <N> sessions, <N> insights

## Actions Taken
- Compacted: <N> entries (dedup=<N>, consolidated=<N>, decayed=<N>, pruned=<N>)
- Flagged: <list of IDs + reasons>
- Supersession chains repaired: <N>
- Embeddings added: <N>
- Conversation insights promoted: <N>
- Conversation entries pruned: <N>

## Contradiction Resolutions
- #<id> vs #<id>: <resolution + rationale>

## Calibration Notes
- Novelty penalty rate: <percent> of recent entries hit 0.3x (threshold: 30%)
- Session diversity: <sessions with >5 memories in active pool>
- Intent snapshot compliance: <count in last 7d> (<assessment>)
- Conversation checkpoints (7d): OPEN=<N> CLOSE=<N> INSIGHT=<N> PRE_TASK=<N> RESEARCH_CLOSE=<N>

## RGF Candidates (for orchestrator review)
- CANDIDATE: <title> — <evidence summary>

## Recommendations
- <free-form observations for orchestrator>
```

## Rubric

See `.GOV/roles/memory_manager/docs/MEMORY_HYGIENE_RUBRIC.md` for scoring criteria when deciding what to flag, prune, collapse, or promote.

## Canonical Reference

`.GOV/roles_shared/docs/GOVERNANCE_MEMORY_GUIDE.md` — the operational guide for the memory system this role manages.
