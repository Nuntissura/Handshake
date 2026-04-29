import path from "node:path";
import { appendJsonlLine, parseJsonlFile } from "../session/session-registry-lib.mjs";
import { governanceRuntimeAbsPath, normalizePath } from "./runtime-paths.mjs";

export const BASELINE_COMPILE_WAIVER_SCHEMA_ID = "hsk.baseline_compile_waiver@1";
export const BASELINE_COMPILE_WAIVER_SCHEMA_VERSION = "baseline_compile_waiver_v1";

function normalizeScalar(value) {
  return String(value || "").trim();
}

function normalizeStatus(value) {
  return normalizeScalar(value).toUpperCase() || "ACTIVE";
}

function normalizePathList(value) {
  if (Array.isArray(value)) return value.map((entry) => normalizePath(entry)).filter(Boolean);
  return normalizeScalar(value)
    .split(/[,\n]/)
    .map((entry) => normalizePath(entry))
    .filter(Boolean);
}

export function baselineWaiverLedgerPath(wpId = "") {
  const normalizedWpId = normalizeScalar(wpId);
  if (!normalizedWpId) throw new Error("baseline waiver ledger requires wpId");
  return governanceRuntimeAbsPath("roles_shared", "WP_COMMUNICATIONS", normalizedWpId, "BASELINE_COMPILE_WAIVERS.jsonl");
}

export function normalizeBaselineCompileWaiver(raw = {}) {
  return {
    schema_id: normalizeScalar(raw.schema_id) || BASELINE_COMPILE_WAIVER_SCHEMA_ID,
    schema_version: normalizeScalar(raw.schema_version) || BASELINE_COMPILE_WAIVER_SCHEMA_VERSION,
    waiver_id: normalizeScalar(raw.waiver_id) || `BCW-${Date.now().toString(36)}`,
    wp_id: normalizeScalar(raw.wp_id),
    status: normalizeStatus(raw.status),
    recorded_at_utc: normalizeScalar(raw.recorded_at_utc) || new Date().toISOString(),
    blocker_command: normalizeScalar(raw.blocker_command),
    failing_files: normalizePathList(raw.failing_files),
    allowed_edit_paths: normalizePathList(raw.allowed_edit_paths),
    allowed_edit_kind: normalizeScalar(raw.allowed_edit_kind) || "BASELINE_COMPILE_REPAIR",
    expiry_condition: normalizeScalar(raw.expiry_condition) || "expires when blocker command passes or final_outcome is recorded",
    operator_authority_ref: normalizeScalar(raw.operator_authority_ref),
    proof_command: normalizeScalar(raw.proof_command),
    final_outcome: normalizeScalar(raw.final_outcome),
  };
}

export function readBaselineCompileWaiverLedger(wpId = "") {
  const filePath = baselineWaiverLedgerPath(wpId);
  const rows = parseJsonlFile(filePath).map((entry) => normalizeBaselineCompileWaiver(entry));
  return { filePath, rows };
}

export function activeBaselineCompileWaiversForWp(wpId = "") {
  return readBaselineCompileWaiverLedger(wpId).rows.filter((row) =>
    row.status === "ACTIVE" && !row.final_outcome
  );
}

function pathCoveredByPattern(repoPath = "", pattern = "") {
  const normalizedPath = normalizePath(repoPath);
  const normalizedPattern = normalizePath(pattern);
  if (!normalizedPath || !normalizedPattern) return false;
  if (normalizedPattern.endsWith("/**")) {
    const prefix = normalizedPattern.slice(0, -3);
    return normalizedPath === prefix || normalizedPath.startsWith(`${prefix}/`);
  }
  return normalizedPath === normalizedPattern;
}

export function activeWaiversForPath(repoPath = "", waivers = []) {
  return (Array.isArray(waivers) ? waivers : [])
    .map((entry) => normalizeBaselineCompileWaiver(entry))
    .filter((entry) => entry.status === "ACTIVE" && !entry.final_outcome)
    .filter((entry) => entry.allowed_edit_paths.some((pattern) => pathCoveredByPattern(repoPath, pattern)));
}

export function evaluateWaiverCoverage({
  paths = [],
  waivers = [],
} = {}) {
  const normalizedPaths = normalizePathList(paths);
  const coverage = normalizedPaths.map((repoPath) => {
    const matchingWaivers = activeWaiversForPath(repoPath, waivers);
    return {
      path: repoPath,
      covered: matchingWaivers.length > 0,
      waiver_ids: matchingWaivers.map((entry) => entry.waiver_id),
    };
  });
  return {
    ok: coverage.every((entry) => entry.covered),
    covered: coverage.filter((entry) => entry.covered),
    uncovered: coverage.filter((entry) => !entry.covered),
    coverage,
  };
}

export function recordBaselineCompileWaiver({
  wpId = "",
  waiver = {},
} = {}) {
  const normalizedWpId = normalizeScalar(wpId || waiver.wp_id);
  if (!normalizedWpId) throw new Error("recordBaselineCompileWaiver requires wpId");
  const entry = normalizeBaselineCompileWaiver({
    ...waiver,
    wp_id: normalizedWpId,
  });
  if (!entry.blocker_command) throw new Error("baseline compile waiver requires blocker_command");
  if (entry.allowed_edit_paths.length === 0) throw new Error("baseline compile waiver requires allowed_edit_paths");
  if (!entry.operator_authority_ref) throw new Error("baseline compile waiver requires operator_authority_ref");
  const filePath = baselineWaiverLedgerPath(normalizedWpId);
  appendJsonlLine(filePath, entry);
  return { filePath, entry };
}

export function displayWaiverLedgerPath(repoRoot = process.cwd(), wpId = "") {
  return normalizePath(path.relative(repoRoot, baselineWaiverLedgerPath(wpId))) || normalizePath(baselineWaiverLedgerPath(wpId));
}
