# Governance Memory System — Operational Guide

Authority: [CX-503K, CX-503K1, CX-503K2, CX-503K3]

## What It Is

A provider-agnostic, cross-session knowledge system that persists governance lessons, failure patterns, and codebase facts across sessions, roles, and WPs. Any model (GPT, Claude, Codex, future providers) that enters a governed session gets relevant context from all previous sessions.

**Memory is supplementary, not authoritative.** Work packets, receipts, and governance ledgers remain the source of truth. Memory helps roles avoid repeating known mistakes.

## Storage

- **Database:** `gov_runtime/roles_shared/GOVERNANCE_MEMORY.db` (SQLite, WAL mode, busy_timeout=5s)
- **Backup:** `gov_runtime/` is included in backup snapshots. `just memory-export` provides JSONL archival; `just memory-import` restores.
- **Dependencies:** SQLite + FTS5 only. No external services required.
- **Optional:** Ollama for vector embeddings (`just memory-embed`). All core functionality works without it.
- **Legacy:** `FAILURE_MEMORY.json` has been migrated into the DB and archived as `.migrated`. The legacy `just failure-memory-record` and `just failure-memory-query` commands still work (they redirect to the DB) but are deprecated. Use `just memory-capture procedural` and `just memory-search` instead.

## Memory Types

| Type | Purpose | Example | Who creates |
|---|---|---|---|
| **procedural** | Fix patterns, workflows, recipes — the fail log | "packet.md must fill CODER_MODEL or pre-work fails" | Extracted from REPAIR receipts; smoketest SMOKE-FIND entries with fix direction; `just memory-capture procedural` |
| **semantic** | Distilled facts, codebase patterns, governance decisions | "Database trait boundary is the required storage access pattern" | Extracted from smoketest SMOKE-CONTROL entries; consolidated from episodic over time; `just memory-capture semantic` |
| **episodic** | Timestamped session events — what happened, when, by whom | "CODER_HANDOFF by CODER on MT-002 at 2026-04-06T18:00Z" | Extracted automatically from receipts (event-driven + batch) |

## Who Gets What at Startup

| Role | What they receive | Section header | Budget |
|---|---|---|---|
| **Coder** | Procedural only (the fail log) | `FAIL LOG:` | 1500 tokens |
| **Validator** | Procedural + semantic (fail log + governance context) | `FAIL LOG:` + `CONTEXT:` | 1500 tokens |
| **Orchestrator** | Full cross-WP memory (all types, governance-weighted) | `GOVERNANCE MEMORY:` | 2000 tokens |

Procedural memories are shown with content snippets (the actual fix recipe). Semantic memories are shown as one-liners. Episodic memories compress into a timeline.

## How Memories Enter the System

**Automatic (no action needed):**
1. **Event-driven extraction** — every `wp-receipt-append` immediately extracts a memory entry for high-signal receipt kinds (STEERING, REPAIR, WORKFLOW_INVALIDITY, SPEC_GAP, CODER_HANDOFF, VALIDATOR_REVIEW) [RGF-126]
2. **Batch extraction at role startup** — `just memory-refresh` runs at every role startup (orchestrator, coder, validator), extracting from all WP receipts and smoketest reviews [RGF-131]
3. **Validator check failures** — when `validator-scan`, `validator-handoff-check`, or `validator-packet-complete` find issues, findings are persisted as procedural memories
4. **Coder post-work failures** — when `just post-work` fails, the failure pattern is captured as a procedural memory

**Manual (role-initiated):**
- `just memory-capture procedural "CRLF handling requires explicit line ending normalization" --scope "sqlite.rs" --wp WP-{ID}`
- `just memory-capture semantic "DateTime imports fail silently when chrono feature flag is missing" --wp WP-{ID}`

## Lifecycle: Score, Inject, Decay, Consolidate

1. **Score** — each memory has an importance value (0-1), access count, and creation timestamp
2. **Inject** — at session startup, candidates are scored by `importance * recency_decay * access_boost * file_scope_match * staleness_factor` and packed into the token budget
3. **Decay** — Ebbinghaus exponential decay: `new_importance = importance * e^(-0.1 * days_since_access)`. Memories below 0.05 threshold are archived
4. **Consolidate** — old episodic memories (>30 days) are grouped by WP and compressed into semantic summaries
5. **Staleness check** — memories referencing files that no longer exist get a 0.5x score penalty [RGF-130]

## Maintenance

Runs automatically at every role startup via `just memory-refresh`:
- **Extraction:** always (idempotent)
- **Dedup:** if >6h since last compaction
- **Full compaction:** if >24h since last compaction (dedup + consolidation + decay + orphan cleanup)

Manual commands:
- `just memory-stats` — DB health overview
- `just memory-compact --dry-run` — preview what maintenance would do
- `just memory-refresh --force-compact` — force a full compaction cycle
- `just memory-flag <id> "<reason>"` — suppress a bad/misleading memory (importance -> 0.1)
- `just memory-patterns` — cross-WP pattern synthesis (recurring failures, systemic issues)

## Key Commands

| Command | Purpose |
|---|---|
| `just memory-search "<query>"` | FTS5 keyword search across all memories |
| `just memory-capture <type> "<insight>"` | Record an insight mid-session (importance 0.7) |
| `just memory-flag <id> "<reason>"` | Suppress a bad memory |
| `just memory-prime <WP-{ID}>` | Preview what a session would receive |
| `just memory-patterns` | Cross-WP pattern synthesis for governance improvement |
| `just memory-stats` | Database health and counts |
| `just memory-refresh` | Extract + staleness-gated maintenance |
| `just memory-compact` | Manual dedup + consolidation + decay |

## Design Principles

- **Provider-agnostic:** Pure SQLite + FTS5. No model-specific APIs. Any model that can call shell commands can use it.
- **Mechanical:** All extraction, scoring, decay, and compaction are rule-based. No LLM required for maintenance.
- **Graceful degradation:** If the DB doesn't exist, injection returns empty. If Ollama is down, hybrid search falls back to FTS5. If extraction fails, the receipt write still succeeds.
- **Supplementary, not authoritative:** Memory never overrides packets, receipts, or governance ledgers. Roles should verify memory claims against current state.
