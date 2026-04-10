import assert from "node:assert/strict";
import test from "node:test";

import {
  MECHANICAL_DECAY_OPTIONS,
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
