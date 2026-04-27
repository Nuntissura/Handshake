export const MECHANICAL_STALENESS_HOURS = 24;
export const MECHANICAL_ACTIVITY_THRESHOLD = 10;
export const MECHANICAL_DECAY_OPTIONS = Object.freeze({
  decayRate: 0.1,
  pruneThreshold: 0,
});

// RGF-254: the ACP-launched intelligent review (judgment-based promotion,
// contradiction resolution, RGF candidate drafting) runs on a separate
// cadence from the mechanical pre-pass. A run that is older than this
// window — or has never happened — surfaces as governance debt at IntVal
// closeout so accumulated one-off captures actually get reviewed instead
// of dead-lettering behind the access-count startup-injection gate.
export const INTELLIGENT_REVIEW_STALENESS_DAYS = 7;
export const INTELLIGENT_REVIEW_STALENESS_MS = INTELLIGENT_REVIEW_STALENESS_DAYS * 24 * 60 * 60 * 1000;

export function evaluateIntelligentReviewStaleness({
  lastRunIso = "",
  now = Date.now(),
  stalenessMs = INTELLIGENT_REVIEW_STALENESS_MS,
} = {}) {
  const trimmed = String(lastRunIso || "").trim();
  if (!trimmed) {
    return {
      status: "MISSING",
      reason: "no recorded intelligent review run",
      last_intelligent_review_iso: null,
      days_since_intelligent_review: null,
    };
  }
  const lastMs = new Date(trimmed).getTime();
  if (!Number.isFinite(lastMs)) {
    return {
      status: "MALFORMED",
      reason: `unparseable last-run timestamp: ${trimmed}`,
      last_intelligent_review_iso: trimmed,
      days_since_intelligent_review: null,
    };
  }
  const elapsedMs = Math.max(0, now - lastMs);
  const days = elapsedMs / (24 * 60 * 60 * 1000);
  if (elapsedMs > stalenessMs) {
    return {
      status: "STALE",
      reason: `last intelligent review ${days.toFixed(1)} days ago (gate: ${INTELLIGENT_REVIEW_STALENESS_DAYS} days)`,
      last_intelligent_review_iso: trimmed,
      days_since_intelligent_review: Number(days.toFixed(2)),
    };
  }
  return {
    status: "FRESH",
    reason: `last intelligent review ${days.toFixed(1)} days ago (within ${INTELLIGENT_REVIEW_STALENESS_DAYS}-day cadence)`,
    last_intelligent_review_iso: trimmed,
    days_since_intelligent_review: Number(days.toFixed(2)),
  };
}

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
