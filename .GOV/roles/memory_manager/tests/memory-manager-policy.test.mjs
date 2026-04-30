import assert from "node:assert/strict";
import test from "node:test";

import {
  INTELLIGENT_REVIEW_STALENESS_DAYS,
  INTELLIGENT_REVIEW_STALENESS_MS,
  MECHANICAL_DECAY_OPTIONS,
  buildActionableFailureCandidates,
  evaluateIntelligentReviewStaleness,
  isAgeConsolidationCandidate,
  isStaleFileScopeCandidate,
  shouldRunMechanicalPass,
} from "../scripts/memory-manager-policy.mjs";

test("mechanical pass skips when staleness gate is not met", () => {
  const result = shouldRunMechanicalPass({
    hoursSinceLastRun: 2,
    newEntriesSinceLastRun: 999,
  });

  assert.deepEqual(result, {
    run: false,
    reason: "last run 2.0h ago (gate: 24h)",
  });
});

test("mechanical pass skips when activity threshold is not met", () => {
  const result = shouldRunMechanicalPass({
    hoursSinceLastRun: 72,
    newEntriesSinceLastRun: 3,
  });

  assert.deepEqual(result, {
    run: false,
    reason: "only 3 new entries since last run (gate: 10)",
  });
});

test("mechanical pass force override runs regardless of gates", () => {
  const result = shouldRunMechanicalPass({
    force: true,
    hoursSinceLastRun: 1,
    newEntriesSinceLastRun: 0,
  });

  assert.deepEqual(result, { run: true, reason: "forced" });
});

test("mechanical decay disables pruning by setting prune threshold to zero", () => {
  assert.equal(MECHANICAL_DECAY_OPTIONS.decayRate, 0.1);
  assert.equal(MECHANICAL_DECAY_OPTIONS.pruneThreshold, 0);
});

test("stale file-scope candidates are report-only when old, unreferenced, and low access", () => {
  assert.equal(isStaleFileScopeCandidate({
    createdAt: "2026-03-01T00:00:00.000Z",
    cutoffIso: "2026-04-02T00:00:00.000Z",
    accessCount: 1,
    existingFilesCount: 0,
  }), true);

  assert.equal(isStaleFileScopeCandidate({
    createdAt: "2026-04-08T00:00:00.000Z",
    cutoffIso: "2026-04-02T00:00:00.000Z",
    accessCount: 1,
    existingFilesCount: 0,
  }), false);
});

test("age consolidation candidates are identified without auto-consolidating", () => {
  assert.equal(isAgeConsolidationCandidate({
    createdAt: "2026-02-01T00:00:00.000Z",
    cutoffIso: "2026-03-10T00:00:00.000Z",
    accessCount: 0,
    importance: 0.2,
  }), true);

  assert.equal(isAgeConsolidationCandidate({
    createdAt: "2026-02-01T00:00:00.000Z",
    cutoffIso: "2026-03-10T00:00:00.000Z",
    accessCount: 4,
    importance: 0.2,
  }), false);
});

test("intelligent review staleness: MISSING when no marker recorded", () => {
  const result = evaluateIntelligentReviewStaleness({
    lastRunIso: "",
    now: new Date("2026-04-27T00:00:00.000Z").getTime(),
  });
  assert.equal(result.status, "MISSING");
  assert.equal(result.last_intelligent_review_iso, null);
  assert.equal(result.days_since_intelligent_review, null);
});

test("intelligent review staleness: FRESH when within window", () => {
  const now = new Date("2026-04-27T00:00:00.000Z").getTime();
  const lastRunIso = new Date(now - 3 * 24 * 60 * 60 * 1000).toISOString();
  const result = evaluateIntelligentReviewStaleness({ lastRunIso, now });
  assert.equal(result.status, "FRESH");
  assert.equal(result.days_since_intelligent_review, 3);
});

test("intelligent review staleness: STALE when past window", () => {
  const now = new Date("2026-04-27T00:00:00.000Z").getTime();
  const lastRunIso = new Date(now - (INTELLIGENT_REVIEW_STALENESS_DAYS + 2) * 24 * 60 * 60 * 1000).toISOString();
  const result = evaluateIntelligentReviewStaleness({ lastRunIso, now });
  assert.equal(result.status, "STALE");
  assert.equal(result.days_since_intelligent_review, INTELLIGENT_REVIEW_STALENESS_DAYS + 2);
});

test("intelligent review staleness: MALFORMED when timestamp is unparseable", () => {
  const result = evaluateIntelligentReviewStaleness({
    lastRunIso: "not-a-real-iso-timestamp",
    now: Date.now(),
  });
  assert.equal(result.status, "MALFORMED");
  assert.equal(result.days_since_intelligent_review, null);
});

test("intelligent review staleness: gate equals INTELLIGENT_REVIEW_STALENESS_MS / day", () => {
  assert.equal(INTELLIGENT_REVIEW_STALENESS_MS, INTELLIGENT_REVIEW_STALENESS_DAYS * 24 * 60 * 60 * 1000);
});

test("actionable failure candidates group repeated role/action failures", () => {
  const candidates = buildActionableFailureCandidates([
    {
      id: 101,
      memory_type: "procedural",
      source_role: "ORCHESTRATOR",
      topic: "repomem open failed under quality gate",
      content: "repomem open failed because the session purpose was under the 80-character quality gate",
    },
    {
      id: 102,
      memory_type: "procedural",
      source_role: "ORCHESTRATOR",
      topic: "SESSION_OPEN quality gate failure",
      content: "repomem open content below 80 chars; use a substantive purpose",
    },
    {
      id: 103,
      memory_type: "procedural",
      source_role: "CODER",
      topic: "unrelated compile failure",
      content: "missing import in product test",
    },
  ]);

  assert.equal(candidates.length, 1);
  assert.equal(candidates[0].role, "ORCHESTRATOR");
  assert.equal(candidates[0].action, "SESSION_OPEN");
  assert.equal(candidates[0].recommended_surface, "BOTH");
  assert.deepEqual(candidates[0].ids, [101, 102]);
});
