# Memory Hygiene Rubric

Decision framework for the Memory Manager. Apply these rules when judging what to flag, prune, collapse, or promote.

## When to FLAG (suppress via `just memory-flag`)

| Condition | Action | Reason |
|---|---|---|
| Memory references a file that no longer exists AND has no general applicability | FLAG | Stale, misleading |
| Two memories contradict each other on the same file_scope | FLAG the older one (unless it is more detailed) | Newer information wins by default |
| Memory content is a generic truism ("always test your code") | FLAG | No actionable value, wastes injection tokens |
| Memory was captured from a session that ended in failure/cancellation | FLAG | Source session's work may have been wrong |
| Memory importance is below 0.15 after decay | FLAG | Below useful threshold, consuming DB budget |
| INTENT snapshot with no meaningful content ("doing work", "starting task") | FLAG | Vague intent has no post-hoc analysis value |
| Procedural memory whose fix pattern is factually wrong (verified against current code) | FLAG | Injecting wrong fix patterns is worse than injecting nothing |

## When to PRUNE (consolidate=1, archived)

| Condition | Action | Reason |
|---|---|---|
| 3+ episodic memories from the same WP older than 30 days | Compact into 1 semantic summary | Reduce noise while preserving knowledge |
| Procedural memory superseded by a newer one (metadata.superseded_by exists) | Verify supersession is valid, then leave consolidated | Already handled by RGF-137 |
| Duplicate memories (same topic, same WP, same type) | Keep highest-importance, prune rest | Dedup should have caught this; clean up |
| Memories with access_count=0 AND age >60 days AND importance <0.3 | Prune | Never accessed, old, low importance = dead weight |
| Mechanical snapshots (PRE_*) older than 14 days with access_count=0 | Prune | Decision context loses value fast if never accessed |
| Session-end flush memories (trust 0.5) older than 30 days with access_count < 2 | Prune | Low-trust summaries that didn't prove useful |
| Any entry >30 days old with access_count < 2 AND importance < 0.4 | Consolidate (age-based) | Old entries that never proved useful — free budget for active knowledge |

### Conversation log pruning

| Condition | Action | Reason |
|---|---|---|
| Session >30 days old with no INSIGHT or RESEARCH_CLOSE entries | Delete all entries for that session_id | Sessions without insights are just OPEN/CLOSE bookkeeping |
| Auto-closed session entries ("auto-closed by new session open") | Delete after 14 days | Marker entries with no real content |
| PRE_TASK/context entries >60 days old in sessions that do have insights | Keep the insights, delete the PRE_TASK entries | Context entries have short value; insights are permanent |
| Sessions with insights | NEVER delete, regardless of age | Insights are the primary value; they may be promoted to semantic memories |

## When to REPAIR

| Condition | Action | Reason |
|---|---|---|
| Superseding memory was itself flagged/pruned | Un-consolidate the original (set consolidated=0) | The chain broke — the superseded memory is the best version again |
| Contradiction flag on two memories where one is clearly correct | FLAG the wrong one, restore importance on the correct one | Contradictions at 0.3 importance starve both; resolve, don't leave in limbo |
| Novelty-penalized entry (0.3x) that is actually distinct from the "similar" one | Manually set importance to correct level via `memory-flag` then re-add | FTS5 match was a false positive |

## When to PROMOTE

### Promote to semantic memory (from conversation_log)

| Condition | Action | Reason |
|---|---|---|
| Same INSIGHT topic appears across 3+ sessions | Promote to semantic memory (importance 0.8, source=conversation-promotion) | Cross-session institutional knowledge |
| Same decision in `decisions` column across 2+ sessions | Promote to semantic memory (importance 0.75, source=conversation-promotion) | Repeated decisions should persist beyond conversation context |

### Promote to RGF candidate (from memory_index)

| Condition | Action | Reason |
|---|---|---|
| Same SMOKE-FIND category appears across 3+ WPs | Draft RGF item for root cause fix | Systemic failure pattern |
| Same REPAIR state transition appears across 3+ WPs | Draft RGF item for workflow tooling | Recurring governance friction |
| Procedural memory has access_count >= 10 | Draft RGF item to codify as governance rule | Pattern is so useful it should be permanent |
| Cluster of 5+ procedural memories about the same file_scope | Draft RGF item for that module's quality | Module is a persistent trouble spot |
| High-value INTENT snapshot that reveals a recurring decision pattern | Draft RGF item to mechanize that decision | Judgment-based decisions that repeat should become mechanical |
| Promoted conversation insight has access_count >= 5 | Draft RGF item to codify as governance rule | Insight has proven useful across sessions and now drives injection |

## When to LEAVE ALONE

| Condition | Action | Reason |
|---|---|---|
| Memory is <7 days old | Skip | Too fresh to judge; let it prove itself |
| Memory has access_count >= 3 AND importance >= 0.5 | Skip | Being used, above threshold, working |
| Semantic memory about architecture/patterns | Skip unless contradicted | Stable knowledge, high value |
| Memory captured manually (`metadata.captured_mid_session=true`) | Skip unless factually wrong | Human chose to save this; respect the signal |
| INTENT snapshots <7 days old | Skip | Recent intent context is valuable for active work |
| Mechanical snapshots (PRE_*) <3 days old | Skip | Active decision context, may still be needed |
| **Operator-reported entries** (`source_artifact=operator-reported`) | **NEVER flag, prune, or let decay below 0.5** | Operator knowledge is the highest-trust source; the recall audit restores these automatically if decay drops them below 0.5 |
| **Fail capture entries** (`source_artifact=memory-capture`) | **NEVER flag or prune unless factually wrong** | Role-captured tool failures are high-value procedural knowledge surfaced via memory-recall; decayed entries are auto-restored by the recall audit |

## Scoring Guide (for competing memories)

When multiple memories compete for the same conceptual slot:
- **Higher access_count wins** — more useful in practice
- **Higher importance wins** — unless importance is artificially high from initial extraction
- **More specific file_scope wins** — "sqlite.rs DateTime fix" > "DateTime issues"
- **Newer supersedes older** — unless the older one is more detailed
- **Mechanical sources win over manual** — receipt_extraction > manual_capture for factual accuracy
- **Higher trust_source wins** — receipt (1.0) > smoketest (0.9) > check (0.8) > manual (0.7) > flush (0.5)

## Calibration Signals (report, don't fix mechanically)

These are observations for the orchestrator, not direct actions:

| Signal | Healthy Range | Concern If |
|---|---|---|
| Novelty penalty rate (entries hitting 0.3x at write) | <20% of recent entries | >30% — novelty scoring may be too aggressive |
| Session diversity (memories per source session) | <5 per session in active pool | >8 — one session dominates injection |
| Trust distribution | receipt > smoketest > check > manual > flush by count | Low-trust sources (flush, manual) outnumber mechanical |
| Intent snapshot count (last 7 days) | 1-5 per active WP | 0 — roles are not following protocol; >20 — noise |
| Embedding coverage | >80% of active entries | <50% — hybrid search degraded to FTS-only |
| Active entry count vs 500 cap | <400 | >450 — approaching forced pruning territory |
| Average importance of active entries | 0.3-0.7 | <0.2 — too much decay; >0.8 — not enough decay |
| Contradiction count | <5 unresolved | >10 — resolve backlog before it poisons injection |
| Operator-reported entries active | stable or growing | Decaying below 0.5 — recall audit should restore; count shrinking signals data loss |
| Recall audit restorations per run | 0 (healthy) | >3 — decay rate may be too aggressive for high-value entries |
| Conversation OPEN:CLOSE ratio (7d) | ~1:1 | OPEN >> CLOSE — models not closing sessions |
| Conversation INSIGHT count (7d) | 1-5 per session | 0 — models not capturing decisions; >10/session — possible noise |
| Promoted conversation insights | growing slowly | 0 after 2+ weeks — promotion rules not triggering; >20 — review for quality |
