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
export const ACTIONABLE_FAILURE_THRESHOLD = 2;

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

function normalizeText(value = "") {
  return String(value || "").toLowerCase().replace(/\s+/g, " ").trim();
}

export function inferActionableFailureAction(entry = {}) {
  const text = normalizeText([
    entry.topic,
    entry.summary,
    entry.content,
  ].filter(Boolean).join(" "));
  if (/repomem.*open|session[-_ ]?open|quality gate/.test(text)) return "SESSION_OPEN";
  if (/powershell|regex|rg |ripgrep|quot|mojibake|unicode/.test(text)) return "TOOLCALLING";
  if (/path|worktree|topology|junction|handshake_main|wt-gov-kernel/.test(text)) return "PATHING";
  if (/fail[-_ ]?capture|process\.exit|failwithmemory/.test(text)) return "GOVERNANCE_SCRIPTING";
  return "PROCEDURAL_FAILURE";
}

export function normalizeActionableFailurePattern(entry = {}) {
  const text = normalizeText([
    entry.topic,
    entry.summary,
    entry.content,
  ].filter(Boolean).join(" "));
  if (/repomem.*open/.test(text) && /80|quality gate|under/.test(text)) {
    return "repomem open content below quality gate";
  }
  if (/powershell/.test(text) && /regex|mojibake|unicode|glyph/.test(text)) {
    return "PowerShell malformed Unicode or regex quoting failure";
  }
  if (/failwithmemory/.test(text) || /fail[-_ ]?capture/.test(text)) {
    return "governance fail-capture wiring or signature failure";
  }
  return text
    .replace(/#\d+/g, "#N")
    .replace(/\bwp-[a-z0-9._-]+/gi, "WP-X")
    .replace(/\b\d{4}-\d{2}-\d{2}[t ][^\s]+/g, "TIMESTAMP")
    .slice(0, 120);
}

export function recommendedSurfaceForActionableFailure({ action = "", pattern = "" } = {}) {
  const normalizedAction = String(action || "").trim().toUpperCase();
  const normalizedPattern = normalizeText(pattern);
  if (normalizedAction === "SESSION_OPEN" && /repomem open/.test(normalizedPattern)) return "BOTH";
  if (["TOOLCALLING", "PATHING", "GOVERNANCE_SCRIPTING"].includes(normalizedAction)) return "STARTUP_BRIEF";
  return "STARTUP_BRIEF";
}

export function buildActionableFailureCandidates(entries = [], { threshold = ACTIONABLE_FAILURE_THRESHOLD } = {}) {
  const groups = new Map();
  for (const entry of entries) {
    const memoryType = String(entry.memory_type || "").toLowerCase();
    if (memoryType && memoryType !== "procedural") continue;
    const action = inferActionableFailureAction(entry);
    const pattern = normalizeActionableFailurePattern(entry);
    if (!pattern) continue;
    const role = String(entry.source_role || entry.role || "UNKNOWN").trim().toUpperCase() || "UNKNOWN";
    const key = `${role}|${action}|${pattern}`;
    if (!groups.has(key)) {
      groups.set(key, {
        role,
        action,
        pattern,
        count: 0,
        ids: [],
        topics: [],
        recommended_surface: recommendedSurfaceForActionableFailure({ action, pattern }),
      });
    }
    const group = groups.get(key);
    group.count += 1;
    if (entry.id !== undefined && entry.id !== null) group.ids.push(entry.id);
    if (entry.topic) group.topics.push(String(entry.topic));
  }
  return [...groups.values()]
    .filter((group) => group.count >= threshold)
    .sort((left, right) => {
      if (right.count !== left.count) return right.count - left.count;
      return `${left.role}/${left.action}`.localeCompare(`${right.role}/${right.action}`);
    });
}
