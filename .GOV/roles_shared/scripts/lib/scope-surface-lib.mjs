import path from "node:path";
import { GOV_ROOT_REPO_REL, resolveRefinementPath, resolveWorkPacketPath, WORK_PACKET_STORAGE_ROOT_REPO_REL } from "./runtime-paths.mjs";

const GOV_ROOT_NORMALIZED = normalizeRepoPath(GOV_ROOT_REPO_REL);
const GOV_DISPLAY_ROOT = ".GOV";
const PRODUCT_ROOT_ALIAS_PREFIX = "../handshake_main/";
const PRODUCT_SURFACE_PREFIXES = ["src/", "app/", "tests/"];
const ROOT_GOVERNANCE_FILES = new Set(["AGENTS.md", "justfile", ".claude", ".github", "orcstart.cmd"]);
const ROOT_GOVERNANCE_PREFIXES = [".claude/", ".github/"];
const LEGACY_SCOPE_PLACEHOLDERS = new Set([
  "NONE",
  "N/A",
  "PATH/TO/FILE",
  "OUT/OF/SCOPE/PATH",
]);
export const SCOPE_DISCIPLINE_PACKET_MIN_VERSION = "2026-03-23";
export const BROAD_TOOL_ALLOWLIST_VALUES = ["NONE", "FORMATTER", "CODEGEN", "SEARCH_REPLACE", "MIGRATION_REWRITE"];

export function normalizeRepoPath(value) {
  let raw = String(value || "").trim().replace(/^`|`$/g, "");
  if (!raw) return "";
  while (raw.startsWith("./") || raw.startsWith(".\\")) raw = raw.slice(2);
  while (raw.startsWith("/")) raw = raw.slice(1);
  while (raw.startsWith("\\")) raw = raw.slice(1);
  const normalized = path
    .normalize(raw)
    .replace(/\\/g, "/")
    .replace(/\/+/g, "/");
  if (normalized === ".") return "";
  return normalized.replace(/^\/+/, "");
}

export function summarizeItemList(items, {
  sampleSize = 8,
} = {}) {
  const values = Array.from(new Set(
    (items || [])
      .map((item) => String(item || "").trim())
      .filter(Boolean),
  ));
  return {
    count: values.length,
    sample: values.slice(0, Math.max(1, sampleSize)),
    remaining_count: Math.max(0, values.length - Math.max(1, sampleSize)),
    values,
  };
}

export function formatBoundedItemList(items, {
  sampleSize = 8,
  noun = "item",
} = {}) {
  const summary = summarizeItemList(items, { sampleSize });
  if (summary.count === 0) {
    return `0 ${noun}(s)`;
  }
  const sampleText = summary.sample.join(", ");
  if (summary.remaining_count > 0) {
    return `${summary.count} ${noun}(s): ${sampleText}, ... (+${summary.remaining_count} more)`;
  }
  return `${summary.count} ${noun}(s): ${sampleText}`;
}

function topLevelLabelRegex(label) {
  return new RegExp(`^\\s*-\\s*(?:\\*\\*)?${escapeRegex(label)}(?:\\*\\*)?\\s*:\\s*$`, "i");
}

function headingRegex(label) {
  return new RegExp(`^#{2,6}\\s+${escapeRegex(label)}\\b`, "i");
}

function escapeRegex(value) {
  return String(value || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function collectIndentedBullets(lines, startIndex, { stopLabels = [] } = {}) {
  const stopRegexes = stopLabels.map((label) => topLevelLabelRegex(label));
  const topLevelLabelRe = /^-\s*(?:\*\*)?[A-Z][A-Z0-9_ ()/.-]*(?:\*\*)?\s*:\s*$/;
  const results = [];

  for (let index = startIndex; index < lines.length; index += 1) {
    const line = lines[index];
    if (stopRegexes.some((re) => re.test(line))) break;
    if (/^##\s+\S/.test(line)) break;
    if (topLevelLabelRe.test(line)) break;
    const match = line.match(/^\s{2,}[-*]\s+(.+?)\s*$/);
    if (match) results.push(match[1].trim());
  }

  return results;
}

function collectHeadingBullets(lines, startIndex) {
  const results = [];
  let inFence = false;

  for (let index = startIndex; index < lines.length; index += 1) {
    const line = lines[index];
    const trimmed = line.trim();
    if (/^```/.test(trimmed)) {
      inFence = !inFence;
      continue;
    }
    if (!inFence && /^#{1,6}\s+\S/.test(line)) break;
    const match = line.match(/^\s*[-*]\s+(.+?)\s*$/);
    if (match) results.push(match[1].trim());
  }

  return results;
}

export function parsePacketScopeList(packetContent, label, { stopLabels = [] } = {}) {
  const lines = String(packetContent || "").split(/\r?\n/);
  const bulletIndex = lines.findIndex((line) => topLevelLabelRegex(label).test(line));
  const bulletResults = bulletIndex === -1 ? [] : collectIndentedBullets(lines, bulletIndex + 1, { stopLabels });
  if (bulletResults.length > 0) return normalizeScopeEntries(bulletResults);

  const headingIndex = lines.findIndex((line) => headingRegex(label).test(line));
  const headingResults = headingIndex === -1 ? [] : collectHeadingBullets(lines, headingIndex + 1);
  return normalizeScopeEntries(headingResults);
}

export function normalizeScopeEntries(entries) {
  const normalized = [];
  for (const entry of entries || []) {
    const value = normalizeRepoPath(entry);
    if (!value) continue;
    if (isPlaceholderScopeEntry(value)) continue;
    normalized.push(value);
  }
  return Array.from(new Set(normalized));
}

export function parsePacketSingleField(packetContent, label) {
  const match = String(packetContent || "").match(
    new RegExp(`^\\s*-\\s*(?:\\*\\*)?${escapeRegex(label)}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi"),
  );
  return match ? match[1].trim() : "";
}

export function isVersionAtLeast(current, minimum) {
  const currentValue = String(current || "").trim();
  const minimumValue = String(minimum || "").trim();
  if (!/^\d{4}-\d{2}-\d{2}$/.test(currentValue) || !/^\d{4}-\d{2}-\d{2}$/.test(minimumValue)) {
    return false;
  }
  return currentValue >= minimumValue;
}

function parseBroadToolAllowlist(rawValue) {
  const raw = String(rawValue || "").trim();
  if (!raw) {
    return {
      raw,
      values: [],
      invalidTokens: [],
      valid: false,
    };
  }

  const tokens = raw
    .split(",")
    .map((value) => value.trim().toUpperCase())
    .filter(Boolean);
  const unique = Array.from(new Set(tokens));
  if (unique.length === 0) {
    return {
      raw,
      values: [],
      invalidTokens: [],
      valid: false,
    };
  }

  const invalidTokens = unique.filter((token) => !BROAD_TOOL_ALLOWLIST_VALUES.includes(token));
  if (invalidTokens.length > 0) {
    return {
      raw,
      values: unique.filter((token) => BROAD_TOOL_ALLOWLIST_VALUES.includes(token)),
      invalidTokens,
      valid: false,
    };
  }

  if (unique.includes("NONE") && unique.length > 1) {
    return {
      raw,
      values: unique,
      invalidTokens: ["NONE_WITH_OTHERS"],
      valid: false,
    };
  }

  return {
    raw,
    values: unique,
    invalidTokens: [],
    valid: true,
  };
}

export function parsePacketScopeDiscipline(packetContent) {
  const touchedFileBudgetRaw = parsePacketSingleField(packetContent, "TOUCHED_FILE_BUDGET");
  const broadToolAllowlistRaw = parsePacketSingleField(packetContent, "BROAD_TOOL_ALLOWLIST");
  const touchedFileBudget = Number.parseInt(touchedFileBudgetRaw, 10);
  const hasTouchedFileBudget = Boolean(touchedFileBudgetRaw);
  const touchedFileBudgetValid = hasTouchedFileBudget && Number.isInteger(touchedFileBudget) && touchedFileBudget >= 1;
  const broadToolAllowlist = parseBroadToolAllowlist(broadToolAllowlistRaw);

  return {
    touchedFileBudgetRaw,
    touchedFileBudget,
    hasTouchedFileBudget,
    touchedFileBudgetValid,
    broadToolAllowlistRaw,
    broadToolAllowlist: broadToolAllowlist.values,
    broadToolAllowlistValid: broadToolAllowlist.valid,
    invalidBroadToolTokens: broadToolAllowlist.invalidTokens,
  };
}

export function scopeDisciplineRequiresEnforcement(packetFormatVersion) {
  return isVersionAtLeast(packetFormatVersion, SCOPE_DISCIPLINE_PACKET_MIN_VERSION);
}

export function isPlaceholderScopeEntry(value) {
  const normalized = normalizeRepoPath(value).toUpperCase();
  if (!normalized) return true;
  if (LEGACY_SCOPE_PLACEHOLDERS.has(normalized)) return true;
  if (/^<FILL/i.test(normalized)) return true;
  if (/^<PENDING>/i.test(normalized)) return true;
  if (/^<UNCLAIMED>/i.test(normalized)) return true;
  if (/^\{.+\}$/.test(normalized)) return true;
  return false;
}

export function hasConcreteScopeEntries(entries) {
  return normalizeScopeEntries(entries).length > 0;
}

export function matchesScopeEntry(filePath, scopeEntry) {
  const candidateVariants = expandComparableScopeVariants(filePath);
  const scopeVariants = expandComparableScopeVariants(scopeEntry);
  if (candidateVariants.length === 0 || scopeVariants.length === 0) return false;

  for (const candidate of candidateVariants) {
    for (let scope of scopeVariants) {
      if (scope.endsWith("/")) scope = scope.slice(0, -1);
      if (candidate === scope || candidate.startsWith(`${scope}/`)) {
        return true;
      }
    }
  }
  return false;
}

export function matchesAnyScopeEntry(filePath, scopeEntries) {
  return normalizeScopeEntries(scopeEntries).some((entry) => matchesScopeEntry(filePath, entry));
}

export function hasScopeOverlap(aEntries, bEntries) {
  const left = normalizeScopeEntries(aEntries);
  const right = normalizeScopeEntries(bEntries);
  for (const leftEntry of left) {
    for (const rightEntry of right) {
      if (matchesScopeEntry(leftEntry, rightEntry) || matchesScopeEntry(rightEntry, leftEntry)) {
        return { left: leftEntry, right: rightEntry };
      }
    }
  }
  return null;
}

export function isGovJunctionPath(filePath) {
  const normalized = normalizeRepoPath(filePath);
  return Boolean(normalized) && (
    normalized === GOV_DISPLAY_ROOT
    || normalized.startsWith(`${GOV_DISPLAY_ROOT}/`)
    || normalized === GOV_ROOT_NORMALIZED
    || normalized.startsWith(`${GOV_ROOT_NORMALIZED}/`)
  );
}

export function isGovernanceOnlyPath(filePath) {
  const normalized = normalizeRepoPath(filePath);
  if (!normalized) return false;
  if (ROOT_GOVERNANCE_FILES.has(normalized)) return true;
  if (ROOT_GOVERNANCE_PREFIXES.some((prefix) => normalized.startsWith(prefix))) return true;
  return isGovJunctionPath(normalized);
}

export function isTransientProofArtifactPath(filePath) {
  const normalized = normalizeRepoPath(filePath);
  if (!normalized) return false;
  const baseName = path.posix.basename(normalized).toLowerCase();
  return /^tmp-[^.].*\.log$/i.test(baseName);
}

export function isRootGovernancePath(filePath) {
  const normalized = normalizeRepoPath(filePath);
  if (!normalized) return false;
  return ROOT_GOVERNANCE_FILES.has(normalized) || ROOT_GOVERNANCE_PREFIXES.some((prefix) => normalized.startsWith(prefix));
}

export function isProductPath(filePath) {
  const normalized = normalizeRepoPath(filePath);
  return ["src/", "app/", "tests/"].some((prefix) => normalized.startsWith(prefix));
}

function expandComparableScopeVariants(value) {
  const normalized = normalizeRepoPath(value);
  if (!normalized) return [];
  const variants = new Set([normalized]);

  if (normalized.startsWith(PRODUCT_ROOT_ALIAS_PREFIX)) {
    const aliasCandidate = normalized.slice(PRODUCT_ROOT_ALIAS_PREFIX.length);
    if (PRODUCT_SURFACE_PREFIXES.some((prefix) => aliasCandidate.startsWith(prefix))) {
      variants.add(aliasCandidate);
    }
  }

  if (PRODUCT_SURFACE_PREFIXES.some((prefix) => normalized.startsWith(prefix))) {
    variants.add(`${PRODUCT_ROOT_ALIAS_PREFIX}${normalized}`);
  }

  return [...variants];
}

export function deriveWpScopeContract({ wpId, packetContent }) {
  const packetResolved = resolveWorkPacketPath(wpId);
  const packetPath = normalizeRepoPath(
    packetResolved?.packetPath || `${WORK_PACKET_STORAGE_ROOT_REPO_REL}/${wpId}.md`,
  );
  const refinementPath = normalizeRepoPath(
    resolveRefinementPath(wpId) || `${GOV_ROOT_REPO_REL}/refinements/${wpId}.md`,
  );
  const inScopePaths = parsePacketScopeList(packetContent, "IN_SCOPE_PATHS", { stopLabels: ["OUT_OF_SCOPE"] });
  const outOfScopePaths = parsePacketScopeList(packetContent, "OUT_OF_SCOPE");
  const companionCandidates = [
    packetPath,
    refinementPath,
    `${GOV_ROOT_REPO_REL}/roles_shared/records/TASK_BOARD.md`,
    `${GOV_ROOT_REPO_REL}/roles_shared/records/SIGNATURE_AUDIT.md`,
  ].map(normalizeRepoPath).filter(Boolean);
  const governanceCompanionPaths = new Set();
  for (const candidate of companionCandidates) {
    governanceCompanionPaths.add(candidate);
    if (candidate === GOV_ROOT_NORMALIZED) {
      governanceCompanionPaths.add(GOV_DISPLAY_ROOT);
      continue;
    }
    if (candidate.startsWith(`${GOV_ROOT_NORMALIZED}/`)) {
      governanceCompanionPaths.add(`${GOV_DISPLAY_ROOT}${candidate.slice(GOV_ROOT_NORMALIZED.length)}`);
    }
  }

  return {
    packetPath,
    refinementPath,
    inScopePaths,
    outOfScopePaths,
    governanceCompanionPaths,
  };
}

export function classifyWpChangedPath(filePath, scopeContract) {
  const normalized = normalizeRepoPath(filePath);
  if (!normalized) {
    return { path: normalized, kind: "EMPTY_PATH", allowed: false };
  }

  if (scopeContract?.governanceCompanionPaths?.has(normalized)) {
    return { path: normalized, kind: "GOVERNANCE_COMPANION", allowed: true };
  }

  if (matchesAnyScopeEntry(normalized, scopeContract?.outOfScopePaths || [])) {
    return { path: normalized, kind: "EXPLICIT_OUT_OF_SCOPE", allowed: false };
  }

  if (isGovJunctionPath(normalized)) {
    return { path: normalized, kind: "GOVERNANCE_JUNCTION_DRIFT", allowed: false };
  }

  if (matchesAnyScopeEntry(normalized, scopeContract?.inScopePaths || [])) {
    return { path: normalized, kind: "IN_SCOPE", allowed: true };
  }

  if (isRootGovernancePath(normalized)) {
    return { path: normalized, kind: "ROOT_GOVERNANCE_OUT_OF_SCOPE", allowed: false };
  }

  if (isProductPath(normalized)) {
    return { path: normalized, kind: "PRODUCT_OUT_OF_SCOPE", allowed: false };
  }

  return { path: normalized, kind: "OUT_OF_SCOPE", allowed: false };
}

export function collectBudgetCountedFiles(changedFiles, scopeContract) {
  const counted = new Set();
  for (const changedFile of changedFiles || []) {
    const classification = classifyWpChangedPath(changedFile, scopeContract);
    if (classification.kind === "IN_SCOPE") {
      counted.add(classification.path);
    }
  }
  return [...counted];
}
