import crypto from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { GOVERNANCE_RUNTIME_ROOT_ABS } from "./runtime-paths.mjs";

export const CHECK_RESULT_SCHEMA_ID = "hsk.check_result@1";
export const CHECK_RESULT_SCHEMA_VERSION = "check_result_v1";
export const CHECK_RESULT_VERDICTS = new Set(["OK", "WARN", "FAIL"]);
export const CHECK_RESULT_SUMMARY_MAX_CHARS = 120;

function normalizeVerdict(verdict) {
  const normalized = String(verdict || "").trim().toUpperCase();
  if (!CHECK_RESULT_VERDICTS.has(normalized)) {
    throw new Error(`createCheckResult: verdict must be one of ${[...CHECK_RESULT_VERDICTS].join(", ")}`);
  }
  return normalized;
}

function normalizeSummary(summary) {
  const normalized = String(summary || "").replace(/\s+/g, " ").trim();
  if (!normalized) {
    throw new Error("createCheckResult: summary is required");
  }
  if (normalized.length > CHECK_RESULT_SUMMARY_MAX_CHARS) {
    throw new Error(`createCheckResult: summary must be <= ${CHECK_RESULT_SUMMARY_MAX_CHARS} characters`);
  }
  return normalized;
}

export function compactCheckSummary(summary, maxChars = CHECK_RESULT_SUMMARY_MAX_CHARS) {
  const normalized = String(summary || "").replace(/\s+/g, " ").trim();
  const limit = Number(maxChars || CHECK_RESULT_SUMMARY_MAX_CHARS);
  if (normalized.length <= limit) return normalized;
  return `${normalized.slice(0, Math.max(0, limit - 3)).trimEnd()}...`;
}

function normalizeDetails(details) {
  if (!details || typeof details !== "object" || Array.isArray(details)) {
    throw new Error("createCheckResult: details must be an object");
  }
  return details;
}

function stableStringify(value) {
  if (Array.isArray(value)) {
    return `[${value.map((entry) => stableStringify(entry)).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function entryIdFor(entry) {
  return crypto.createHash("sha256").update(stableStringify({
    check: entry.check,
    wp_id: entry.wp_id,
    phase: entry.phase,
    verdict: entry.verdict,
    summary: entry.summary,
    details: entry.details,
  })).digest("hex").slice(0, 16);
}

export function createCheckResult({ verdict, summary, details = {} } = {}) {
  return {
    schema_id: CHECK_RESULT_SCHEMA_ID,
    schema_version: CHECK_RESULT_SCHEMA_VERSION,
    verdict: normalizeVerdict(verdict),
    summary: normalizeSummary(summary),
    details: normalizeDetails(details),
  };
}

export function formatCheckResultSummary(result) {
  const normalized = createCheckResult(result);
  return `${normalized.verdict} | ${normalized.summary}`;
}

export function formatVerboseCheckDetails(entry) {
  return JSON.stringify(entry, null, 2);
}

export function checkDetailsLogPath({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS } = {}) {
  const root = path.resolve(runtimeRootAbs);
  const normalizedWpId = String(wpId || "").trim();
  if (!normalizedWpId) {
    return path.join(root, "check_details.jsonl");
  }
  return path.join(root, "roles_shared", "WP_COMMUNICATIONS", normalizedWpId, "check_details.jsonl");
}

export function appendCheckDetails({
  check,
  wpId = "",
  phase = "",
  result,
  timestamp = new Date().toISOString(),
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const normalizedCheck = String(check || "").trim();
  if (!normalizedCheck) {
    throw new Error("appendCheckDetails: check is required");
  }
  const normalizedResult = createCheckResult(result);
  const entry = {
    schema_id: "hsk.check_detail@1",
    schema_version: "check_detail_v1",
    timestamp,
    check: normalizedCheck,
    wp_id: String(wpId || "").trim() || null,
    phase: String(phase || "").trim() || null,
    verdict: normalizedResult.verdict,
    summary: normalizedResult.summary,
    details: normalizedResult.details,
  };
  entry.entry_id = entryIdFor(entry);

  const logPath = checkDetailsLogPath({ wpId, runtimeRootAbs });
  fs.mkdirSync(path.dirname(logPath), { recursive: true });
  if (fs.existsSync(logPath)) {
    const existing = fs.readFileSync(logPath, "utf8")
      .split(/\r?\n/)
      .filter(Boolean)
      .some((line) => {
        try {
          return JSON.parse(line).entry_id === entry.entry_id;
        } catch {
          return false;
        }
      });
    if (existing) {
      return { appended: false, logPath, entry };
    }
  }
  fs.appendFileSync(logPath, `${JSON.stringify(entry)}\n`, "utf8");
  return { appended: true, logPath, entry };
}

export function recordCheckResult({
  check,
  wpId = "",
  phase = "",
  verdict,
  summary,
  details = {},
  timestamp = new Date().toISOString(),
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const result = createCheckResult({
    verdict,
    summary: compactCheckSummary(summary),
    details,
  });
  const writeResult = appendCheckDetails({
    check,
    wpId,
    phase,
    result,
    timestamp,
    runtimeRootAbs,
  });
  return {
    result,
    writeResult,
    summaryLine: formatCheckResultSummary(result),
  };
}

function ensureTrailingNewline(value = "") {
  const text = String(value || "");
  return text.endsWith("\n") ? text : `${text}\n`;
}

function outputLines(value = "") {
  return String(value || "").split(/\r?\n/).filter((line) => line.length > 0);
}

export function runSubprocessCheckStep({
  check,
  scriptPath,
  args = [],
  cwd = process.cwd(),
  env = process.env,
  wpId = "",
  phase = "",
  runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS,
} = {}) {
  const normalizedCheck = String(check || "").trim();
  if (!normalizedCheck) {
    throw new Error("runSubprocessCheckStep: check is required");
  }
  const normalizedScriptPath = String(scriptPath || "").trim();
  if (!normalizedScriptPath) {
    throw new Error("runSubprocessCheckStep: scriptPath is required");
  }

  const result = spawnSync(process.execPath, [normalizedScriptPath, ...args], {
    cwd,
    env,
    encoding: "utf8",
  });
  const status = result.status ?? 1;
  const stdout = String(result.stdout || "");
  const stderr = String(result.stderr || "");
  const output = ensureTrailingNewline(`${stdout}${stderr}`.trimEnd());
  const ok = status === 0;
  const recorded = recordCheckResult({
    check: normalizedCheck,
    wpId,
    phase,
    verdict: ok ? "OK" : "FAIL",
    summary: `${normalizedCheck} ${ok ? "ok" : "failed"}`,
    details: {
      script_path: normalizedScriptPath,
      args: args.map((arg) => String(arg)),
      cwd: path.resolve(cwd || process.cwd()),
      exit_status: status,
      signal: result.signal || null,
      stdout,
      stderr,
      output_lines: outputLines(output),
      error: result.error ? String(result.error?.message || result.error) : null,
    },
    runtimeRootAbs,
  });

  return {
    ok,
    status,
    signal: result.signal || null,
    stdout,
    stderr,
    output,
    ...recorded,
  };
}

export function readCheckDetails({ wpId = "", runtimeRootAbs = GOVERNANCE_RUNTIME_ROOT_ABS, limit = 50 } = {}) {
  const logPath = checkDetailsLogPath({ wpId, runtimeRootAbs });
  if (!fs.existsSync(logPath)) {
    return [];
  }
  const rows = fs.readFileSync(logPath, "utf8")
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
  return rows.slice(Math.max(0, rows.length - Number(limit || 50)));
}
