export const MECHANICAL_STALENESS_HOURS = 24;
export const MECHANICAL_ACTIVITY_THRESHOLD = 10;
export const MECHANICAL_DECAY_OPTIONS = Object.freeze({
  decayRate: 0.1,
  pruneThreshold: 0,
});

export function shouldRunMechanicalPass({
  force = false,
  hoursSinceLastRun = Infinity,
  newEntriesSinceLastRun = 0,
  stalenessHours = MECHANICAL_STALENESS_HOURS,
  activityThreshold = MECHANICAL_ACTIVITY_THRESHOLD,
} = {}) {
  if (force) {
    return { run: true, reason: "forced" };
  }
  if (hoursSinceLastRun < stalenessHours) {
    return {
      run: false,
      reason: `last run ${hoursSinceLastRun.toFixed(1)}h ago (gate: ${stalenessHours}h)`,
    };
  }
  if (newEntriesSinceLastRun < activityThreshold) {
    return {
      run: false,
      reason: `only ${newEntriesSinceLastRun} new entries since last run (gate: ${activityThreshold})`,
    };
  }
  return { run: true, reason: "staleness gate passed" };
}

function isoToMs(value) {
  const ms = new Date(String(value || "")).getTime();
  return Number.isFinite(ms) ? ms : NaN;
}

export function isStaleFileScopeCandidate({
  createdAt = "",
  cutoffIso = "",
  accessCount = 0,
  existingFilesCount = 0,
} = {}) {
  const createdMs = isoToMs(createdAt);
  const cutoffMs = isoToMs(cutoffIso);
  if (!Number.isFinite(createdMs) || !Number.isFinite(cutoffMs)) return false;
  return createdMs < cutoffMs && existingFilesCount === 0 && Number(accessCount) < 2;
}

export function isAgeConsolidationCandidate({
  createdAt = "",
  cutoffIso = "",
  accessCount = 0,
  importance = 0,
} = {}) {
  const createdMs = isoToMs(createdAt);
  const cutoffMs = isoToMs(cutoffIso);
  if (!Number.isFinite(createdMs) || !Number.isFinite(cutoffMs)) return false;
  return createdMs < cutoffMs && Number(accessCount) < 2 && Number(importance) < 0.4;
}
