import fs from "node:fs";
import path from "node:path";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
  GOVERNANCE_RUNTIME_ROOT_ENV_VAR,
  PRODUCT_RUNTIME_ROOT_ENV_VAR,
} from "../lib/runtime-paths.mjs";
import { writeJsonFile } from "./session-registry-lib.mjs";

export const WP_TOKEN_USAGE_SCHEMA_ID = "hsk.wp_token_usage@1";
export const WP_TOKEN_USAGE_SCHEMA_VERSION = "wp_token_usage_v1";
const OUTPUT_SCAN_SAMPLE_SIZE = 8;

function nowIso() {
  return new Date().toISOString();
}

function normalizeText(value) {
  return String(value || "").trim();
}

function normalizeCount(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric < 0) return 0;
  return Math.trunc(numeric);
}

function normalizeRole(value) {
  return normalizeText(value).toUpperCase() || "UNKNOWN";
}

function defaultUsageTotals() {
  return {
    input_tokens: 0,
    cached_input_tokens: 0,
    output_tokens: 0,
  };
}

function defaultSummary() {
  return {
    command_count: 0,
    turn_count: 0,
    usage_totals: defaultUsageTotals(),
  };
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function addUsageTotals(target, increment) {
  target.input_tokens += normalizeCount(increment?.input_tokens);
  target.cached_input_tokens += normalizeCount(increment?.cached_input_tokens);
  target.output_tokens += normalizeCount(increment?.output_tokens);
}

function usageTotalsEqual(left = {}, right = {}) {
  return normalizeCount(left.input_tokens) === normalizeCount(right.input_tokens)
    && normalizeCount(left.cached_input_tokens) === normalizeCount(right.cached_input_tokens)
    && normalizeCount(left.output_tokens) === normalizeCount(right.output_tokens);
}

function sampleList(values = [], limit = OUTPUT_SCAN_SAMPLE_SIZE) {
  return [...values]
    .map((value) => normalizeText(value))
    .filter(Boolean)
    .sort((left, right) => left.localeCompare(right))
    .slice(0, limit);
}

function normalizeTurnUsageEntry(entry = {}) {
  return {
    timestamp: normalizeText(entry.timestamp),
    input_tokens: normalizeCount(entry.input_tokens),
    cached_input_tokens: normalizeCount(entry.cached_input_tokens),
    output_tokens: normalizeCount(entry.output_tokens),
  };
}

function normalizeCommandEntry(entry = {}) {
  const usageTotals = defaultUsageTotals();
  addUsageTotals(usageTotals, entry.usage_totals);
  const turnUsage = Array.isArray(entry.turn_usage)
    ? entry.turn_usage.map((item) => normalizeTurnUsageEntry(item))
    : [];
  if (turnUsage.length > 0) {
    usageTotals.input_tokens = 0;
    usageTotals.cached_input_tokens = 0;
    usageTotals.output_tokens = 0;
    for (const usageEntry of turnUsage) addUsageTotals(usageTotals, usageEntry);
  }
  return {
    command_id: normalizeText(entry.command_id),
    command_kind: normalizeText(entry.command_kind).toUpperCase() || "UNKNOWN",
    role: normalizeRole(entry.role),
    session_key: normalizeText(entry.session_key),
    session_thread_id: normalizeText(entry.session_thread_id),
    selected_model: normalizeText(entry.selected_model),
    reasoning_config_value: normalizeText(entry.reasoning_config_value),
    status: normalizeText(entry.status).toUpperCase() || "UNKNOWN",
    processed_at: normalizeText(entry.processed_at),
    output_jsonl_file: normalizeText(entry.output_jsonl_file).replace(/\\/g, "/"),
    turn_count: turnUsage.length,
    usage_totals: usageTotals,
    turn_usage: turnUsage,
  };
}

function summarizeCommands(commandEntries = []) {
  const summary = defaultSummary();
  for (const command of commandEntries) {
    summary.command_count += 1;
    summary.turn_count += normalizeCount(command.turn_count);
    addUsageTotals(summary.usage_totals, command.usage_totals);
  }
  return summary;
}

function buildRoleTotals(commandEntries = []) {
  const roleTotals = {};
  for (const command of commandEntries) {
    const role = normalizeRole(command.role);
    if (!roleTotals[role]) {
      roleTotals[role] = defaultSummary();
    }
    roleTotals[role].command_count += 1;
    roleTotals[role].turn_count += normalizeCount(command.turn_count);
    addUsageTotals(roleTotals[role].usage_totals, command.usage_totals);
  }
  return roleTotals;
}

function stableSortCommands(commandEntries = []) {
  return [...commandEntries].sort((left, right) =>
    String(left.processed_at || "").localeCompare(String(right.processed_at || ""))
    || String(left.command_id || "").localeCompare(String(right.command_id || ""))
  );
}

function inferRoleFromOutputDirName(dirName, wpId) {
  const suffix = `_${wpId}`;
  if (!String(dirName || "").endsWith(suffix)) return "";
  return normalizeRole(String(dirName || "").slice(0, -suffix.length));
}

function resolveGovernanceRuntimeRootForRepo(repoRoot) {
  const directRuntimeRoot = normalizeText(process.env[GOVERNANCE_RUNTIME_ROOT_ENV_VAR]);
  if (directRuntimeRoot) return path.resolve(directRuntimeRoot);

  const productRuntimeRoot = normalizeText(process.env[PRODUCT_RUNTIME_ROOT_ENV_VAR]);
  if (productRuntimeRoot) return path.resolve(productRuntimeRoot, "repo-governance");

  if (normalizeText(GOVERNANCE_RUNTIME_ROOT_ABS)) {
    return path.resolve(GOVERNANCE_RUNTIME_ROOT_ABS);
  }

  return path.resolve(repoRoot, "..", "gov_runtime");
}

function governanceRuntimeFileForRepo(repoRoot, ...segments) {
  return path.resolve(resolveGovernanceRuntimeRootForRepo(repoRoot), "roles_shared", ...segments);
}

function parseRawOutputCommand(outputJsonlFile, role, repoRoot) {
  const turnUsage = [];
  let threadId = "";
  let sessionKey = "";
  let commandKind = "";
  let status = "";
  let processedAt = "";
  let commandId = path.basename(outputJsonlFile, path.extname(outputJsonlFile));

  const lines = fs.readFileSync(outputJsonlFile, "utf8")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);

  for (const line of lines) {
    let event;
    try {
      event = JSON.parse(line);
    } catch {
      continue;
    }

    const eventType = normalizeText(event?.type);
    if (eventType === "thread.started" && !threadId) {
      threadId = normalizeText(event.thread_id);
    }
    if (!sessionKey) {
      sessionKey = normalizeText(event.session_id);
    }
    if (!commandKind) {
      commandKind = normalizeText(event.command_kind).toUpperCase();
    }
    if (!status && eventType.startsWith("control.")) {
      status = normalizeText(event.status).toUpperCase();
    }
    if (!processedAt && normalizeText(event.timestamp)) {
      processedAt = normalizeText(event.timestamp);
    }
    if (normalizeText(event.command_id)) {
      commandId = normalizeText(event.command_id);
    }
    if (eventType !== "turn.completed") continue;
    turnUsage.push(normalizeTurnUsageEntry({
      timestamp: event.timestamp,
      input_tokens: event?.usage?.input_tokens,
      cached_input_tokens: event?.usage?.cached_input_tokens,
      output_tokens: event?.usage?.output_tokens,
    }));
  }

  return normalizeCommandEntry({
    command_id: commandId,
    command_kind: commandKind || "UNKNOWN",
    role,
    session_key: sessionKey,
    session_thread_id: threadId,
    status: status || "UNKNOWN",
    processed_at: processedAt || new Date(fs.statSync(outputJsonlFile).mtimeMs).toISOString(),
    output_jsonl_file: path.relative(repoRoot, outputJsonlFile),
    turn_usage: turnUsage,
  });
}

export function parseUsageFromOutputJsonl(outputJsonlFile) {
  const usageTotals = defaultUsageTotals();
  const turnUsage = [];
  let threadId = "";

  if (!outputJsonlFile || !fs.existsSync(outputJsonlFile)) {
    return {
      threadId,
      turnCount: 0,
      usageTotals,
      turnUsage,
    };
  }

  const command = parseRawOutputCommand(outputJsonlFile, "UNKNOWN", process.cwd());
  for (const usageEntry of command.turn_usage) {
    turnUsage.push(usageEntry);
    addUsageTotals(usageTotals, usageEntry);
  }
  threadId = normalizeText(command.session_thread_id);

  return {
    threadId,
    turnCount: turnUsage.length,
    usageTotals,
    turnUsage,
  };
}

export function scanWpSessionOutputCommands(repoRoot, wpId) {
  const outputRoot = governanceRuntimeFileForRepo(repoRoot, "SESSION_CONTROL_OUTPUTS");
  if (!fs.existsSync(outputRoot)) {
    return {
      output_root: outputRoot.replace(/\\/g, "/"),
      directories: [],
      commands: [],
    };
  }

  const directories = fs.readdirSync(outputRoot, { withFileTypes: true })
    .filter((dirent) => dirent.isDirectory())
    .map((dirent) => ({
      dirName: dirent.name,
      absPath: path.join(outputRoot, dirent.name),
      role: inferRoleFromOutputDirName(dirent.name, wpId),
    }))
    .filter((entry) => entry.role);

  const commands = [];
  for (const directory of directories.sort((left, right) => left.dirName.localeCompare(right.dirName))) {
    const files = fs.readdirSync(directory.absPath, { withFileTypes: true })
      .filter((dirent) => dirent.isFile() && dirent.name.endsWith(".jsonl"))
      .map((dirent) => path.join(directory.absPath, dirent.name))
      .sort((left, right) => left.localeCompare(right));
    for (const filePath of files) {
      commands.push(parseRawOutputCommand(filePath, directory.role, repoRoot));
    }
  }

  return {
    output_root: outputRoot.replace(/\\/g, "/"),
    directories: directories
      .map((entry) => path.relative(repoRoot, entry.absPath).replace(/\\/g, "/"))
      .sort((left, right) => left.localeCompare(right)),
    commands,
  };
}

function buildLedgerHealth(trackedCommands = [], rawCommands = []) {
  if (trackedCommands.length === 0 && rawCommands.length === 0) {
    return {
      status: "NO_OUTPUTS",
      reason: "No tracked commands or raw session output files were found for this WP.",
      tracked_command_count: 0,
      raw_output_command_count: 0,
      missing_tracked_command_count: 0,
      stale_tracked_command_count: 0,
      summary_match: true,
      missing_tracked_command_ids_sample: [],
      stale_tracked_command_ids_sample: [],
    };
  }

  const trackedIds = new Set(trackedCommands.map((entry) => entry.command_id).filter(Boolean));
  const rawIds = new Set(rawCommands.map((entry) => entry.command_id).filter(Boolean));
  const missingTracked = [...rawIds].filter((commandId) => !trackedIds.has(commandId));
  const staleTracked = [...trackedIds].filter((commandId) => !rawIds.has(commandId));
  const trackedSummary = summarizeCommands(trackedCommands);
  const rawSummary = summarizeCommands(rawCommands);
  const summaryMatch =
    trackedSummary.command_count === rawSummary.command_count
    && trackedSummary.turn_count === rawSummary.turn_count
    && usageTotalsEqual(trackedSummary.usage_totals, rawSummary.usage_totals);

  const status = missingTracked.length === 0 && staleTracked.length === 0 && summaryMatch
    ? "MATCH"
    : "DRIFT";

  const reasons = [];
  if (missingTracked.length > 0) {
    reasons.push(`${missingTracked.length} raw output command(s) are not represented in the tracked ledger`);
  }
  if (staleTracked.length > 0) {
    reasons.push(`${staleTracked.length} tracked command(s) no longer have raw output files`);
  }
  if (!summaryMatch) {
    reasons.push("tracked command totals do not match raw turn.completed usage");
  }

  return {
    status,
    reason: reasons.join("; ") || "Tracked ledger matches raw session output usage.",
    tracked_command_count: trackedSummary.command_count,
    raw_output_command_count: rawSummary.command_count,
    missing_tracked_command_count: missingTracked.length,
    stale_tracked_command_count: staleTracked.length,
    summary_match: summaryMatch,
    missing_tracked_command_ids_sample: sampleList(missingTracked),
    stale_tracked_command_ids_sample: sampleList(staleTracked),
  };
}

export function resolveWpTokenUsagePath(repoRoot, wpId) {
  return governanceRuntimeFileForRepo(repoRoot, "WP_TOKEN_USAGE", `${wpId}.json`);
}

export function defaultWpTokenUsageLedger(wpId) {
  return {
    schema_id: WP_TOKEN_USAGE_SCHEMA_ID,
    schema_version: WP_TOKEN_USAGE_SCHEMA_VERSION,
    wp_id: normalizeText(wpId),
    updated_at: nowIso(),
    summary_source: "TRACKED_COMMAND_LEDGER",
    summary: defaultSummary(),
    role_totals: {},
    tracked_summary: defaultSummary(),
    tracked_role_totals: {},
    raw_scan: {
      output_root: "",
      directories: [],
      summary: defaultSummary(),
      role_totals: {},
    },
    ledger_health: buildLedgerHealth([], []),
    commands: [],
  };
}

export function normalizeWpTokenUsageLedger(raw, wpId = "", { repoRoot = "" } = {}) {
  const ledger = {
    ...defaultWpTokenUsageLedger(wpId || raw?.wp_id),
    ...(raw && typeof raw === "object" ? raw : {}),
  };
  const trackedCommands = Array.isArray(raw?.commands)
    ? stableSortCommands(raw.commands.map((entry) => normalizeCommandEntry(entry)).filter((entry) => entry.command_id))
    : [];

  const rawScan = repoRoot
    ? scanWpSessionOutputCommands(repoRoot, normalizeText(ledger.wp_id || wpId))
    : { output_root: "", directories: [], commands: [] };
  const rawCommands = stableSortCommands(rawScan.commands || []);
  const trackedSummary = summarizeCommands(trackedCommands);
  const trackedRoleTotals = buildRoleTotals(trackedCommands);
  const rawSummary = summarizeCommands(rawCommands);
  const rawRoleTotals = buildRoleTotals(rawCommands);
  const authoritativeCommands = rawCommands.length > 0 ? rawCommands : trackedCommands;

  ledger.wp_id = normalizeText(ledger.wp_id || wpId);
  ledger.commands = trackedCommands;
  ledger.tracked_summary = trackedSummary;
  ledger.tracked_role_totals = trackedRoleTotals;
  ledger.raw_scan = {
    output_root: normalizeText(rawScan.output_root),
    directories: Array.isArray(rawScan.directories) ? rawScan.directories : [],
    summary: rawSummary,
    role_totals: rawRoleTotals,
  };
  ledger.summary_source = rawCommands.length > 0 ? "RAW_OUTPUT_SCAN" : "TRACKED_COMMAND_LEDGER";
  ledger.summary = summarizeCommands(authoritativeCommands);
  ledger.role_totals = buildRoleTotals(authoritativeCommands);
  ledger.ledger_health = buildLedgerHealth(trackedCommands, rawCommands);
  return ledger;
}

export function readWpTokenUsageLedger(repoRoot, wpId) {
  const filePath = resolveWpTokenUsagePath(repoRoot, wpId);
  if (!fs.existsSync(filePath)) {
    return {
      filePath,
      ledger: normalizeWpTokenUsageLedger(defaultWpTokenUsageLedger(wpId), wpId, { repoRoot }),
    };
  }
  return {
    filePath,
    ledger: normalizeWpTokenUsageLedger(JSON.parse(fs.readFileSync(filePath, "utf8")), wpId, { repoRoot }),
  };
}

export function syncWpTokenUsageLedger(repoRoot, result = {}, {
  session = null,
} = {}) {
  const wpId = normalizeText(result.wp_id);
  if (!wpId) {
    throw new Error("syncWpTokenUsageLedger requires result.wp_id");
  }

  const { filePath, ledger } = readWpTokenUsageLedger(repoRoot, wpId);
  const outputJsonlFile = normalizeText(result.output_jsonl_file)
    ? path.resolve(repoRoot, normalizeText(result.output_jsonl_file))
    : "";
  const usage = parseUsageFromOutputJsonl(outputJsonlFile);
  const nextEntry = normalizeCommandEntry({
    command_id: result.command_id,
    command_kind: result.command_kind,
    role: result.role,
    session_key: result.session_key,
    session_thread_id: result.thread_id || session?.session_thread_id || usage.threadId,
    selected_model: session?.requested_model || "",
    reasoning_config_value: session?.reasoning_config_value || "",
    status: result.status,
    processed_at: result.processed_at || nowIso(),
    output_jsonl_file: normalizeText(result.output_jsonl_file),
    turn_count: usage.turnCount,
    usage_totals: usage.usageTotals,
    turn_usage: usage.turnUsage,
  });

  const commandMap = new Map((ledger.commands || []).map((entry) => [entry.command_id, clone(entry)]));
  commandMap.set(nextEntry.command_id, nextEntry);
  const nextLedger = normalizeWpTokenUsageLedger({
    ...ledger,
    updated_at: nowIso(),
    commands: Array.from(commandMap.values()),
  }, wpId, { repoRoot });
  nextLedger.updated_at = nowIso();

  writeJsonFile(filePath, nextLedger);
  return {
    filePath,
    ledger: nextLedger,
    command: nextEntry,
  };
}
