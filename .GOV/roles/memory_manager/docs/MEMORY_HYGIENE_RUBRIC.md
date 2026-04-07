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

## When to REPAIR

| Condition | Action | Reason |
|---|---|---|
| Superseding memory was itself flagged/pruned | Un-consolidate the original (set consolidated=0) | The chain broke — the superseded memory is the best version again |
| Contradiction flag on two memories where one is clearly correct | FLAG the wrong one, restore importance on the correct one | Contradictions at 0.3 importance starve both; resolve, don't leave in limbo |
| Novelty-penalized entry (0.3x) that is actually distinct from the "similar" one | Manually set importance to correct level via `memory-flag` then re-add | FTS5 match was a false positive |

## When to PROMOTE (draft as RGF candidate)

| Condition | Action | Reason |
|---|---|---|
| Same SMOKE-FIND category appears across 3+ WPs | Draft RGF item for root cause fix | Systemic failure pattern |
| Same REPAIR state transition appears across 3+ WPs | Draft RGF item for workflow tooling | Recurring governance friction |
| Procedural memory has access_count >= 10 | Draft RGF item to codify as governance rule | Pattern is so useful it should be permanent |
| Cluster of 5+ procedural memories about the same file_scope | Draft RGF item for that module's quality | Module is a persistent trouble spot |
| High-value INTENT snapshot that reveals a recurring decision pattern | Draft RGF item to mechanize that decision | Judgment-based decisions that repeat should become mechanical |

## When to LEAVE ALONE

| Condition | Action | Reason |
|---|---|---|
| Memory is <7 days old | Skip | Too fresh to judge; let it prove itself |
| Memory has access_count >= 3 AND importance >= 0.5 | Skip | Being used, above threshold, working |
| Semantic memory about architecture/patterns | Skip unless contradicted | Stable knowledge, high value |
| Memory captured manually (`metadata.captured_mid_session=true`) | Skip unless factually wrong | Human chose to save this; respect the signal |
| INTENT snapshots <7 days old | Skip | Recent intent context is valuable for active work |
| Mechanical snapshots (PRE_*) <3 days old | Skip | Active decision context, may still be needed |

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
