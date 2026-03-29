import fs from "node:fs";
import path from "node:path";
import { SHARED_GOV_WP_TOKEN_USAGE_ROOT } from "../lib/runtime-paths.mjs";
import { writeJsonFile } from "./session-registry-lib.mjs";

export const WP_TOKEN_USAGE_SCHEMA_ID = "hsk.wp_token_usage@1";
export const WP_TOKEN_USAGE_SCHEMA_VERSION = "wp_token_usage_v1";

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

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function addUsageTotals(target, increment) {
  target.input_tokens += normalizeCount(increment?.input_tokens);
  target.cached_input_tokens += normalizeCount(increment?.cached_input_tokens);
  target.output_tokens += normalizeCount(increment?.output_tokens);
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
  const summary = {
    command_count: 0,
    turn_count: 0,
    usage_totals: defaultUsageTotals(),
  };
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
      roleTotals[role] = {
        command_count: 0,
        turn_count: 0,
        usage_totals: defaultUsageTotals(),
      };
    }
    roleTotals[role].command_count += 1;
    roleTotals[role].turn_count += normalizeCount(command.turn_count);
    addUsageTotals(roleTotals[role].usage_totals, command.usage_totals);
  }
  return roleTotals;
}

export function resolveWpTokenUsagePath(repoRoot, wpId) {
  return path.resolve(repoRoot, SHARED_GOV_WP_TOKEN_USAGE_ROOT, `${wpId}.json`);
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
    if (String(event?.type || "").trim() === "thread.started" && !threadId) {
      threadId = normalizeText(event.thread_id);
    }
    if (String(event?.type || "").trim() !== "turn.completed") continue;
    const usageEntry = normalizeTurnUsageEntry({
      timestamp: event.timestamp,
      input_tokens: event?.usage?.input_tokens,
      cached_input_tokens: event?.usage?.cached_input_tokens,
      output_tokens: event?.usage?.output_tokens,
    });
    turnUsage.push(usageEntry);
    addUsageTotals(usageTotals, usageEntry);
  }

  return {
    threadId,
    turnCount: turnUsage.length,
    usageTotals,
    turnUsage,
  };
}

export function defaultWpTokenUsageLedger(wpId) {
  return {
    schema_id: WP_TOKEN_USAGE_SCHEMA_ID,
    schema_version: WP_TOKEN_USAGE_SCHEMA_VERSION,
    wp_id: normalizeText(wpId),
    updated_at: nowIso(),
    summary: {
      command_count: 0,
      turn_count: 0,
      usage_totals: defaultUsageTotals(),
    },
    role_totals: {},
    commands: [],
  };
}

export function normalizeWpTokenUsageLedger(raw, wpId = "") {
  const ledger = {
    ...defaultWpTokenUsageLedger(wpId || raw?.wp_id),
    ...(raw && typeof raw === "object" ? raw : {}),
  };
  const commands = Array.isArray(raw?.commands)
    ? raw.commands.map((entry) => normalizeCommandEntry(entry)).filter((entry) => entry.command_id)
    : [];
  ledger.wp_id = normalizeText(ledger.wp_id || wpId);
  ledger.commands = commands.sort((left, right) =>
    String(left.processed_at || "").localeCompare(String(right.processed_at || ""))
    || String(left.command_id || "").localeCompare(String(right.command_id || ""))
  );
  ledger.summary = summarizeCommands(ledger.commands);
  ledger.role_totals = buildRoleTotals(ledger.commands);
  return ledger;
}

export function readWpTokenUsageLedger(repoRoot, wpId) {
  const filePath = resolveWpTokenUsagePath(repoRoot, wpId);
  if (!fs.existsSync(filePath)) return { filePath, ledger: defaultWpTokenUsageLedger(wpId) };
  return {
    filePath,
    ledger: normalizeWpTokenUsageLedger(JSON.parse(fs.readFileSync(filePath, "utf8")), wpId),
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

  const commandMap = new Map(ledger.commands.map((entry) => [entry.command_id, clone(entry)]));
  commandMap.set(nextEntry.command_id, nextEntry);
  const nextLedger = normalizeWpTokenUsageLedger({
    ...ledger,
    updated_at: nowIso(),
    commands: Array.from(commandMap.values()),
  }, wpId);
  nextLedger.updated_at = nowIso();

  writeJsonFile(filePath, nextLedger);
  return {
    filePath,
    ledger: nextLedger,
    command: nextEntry,
  };
}
