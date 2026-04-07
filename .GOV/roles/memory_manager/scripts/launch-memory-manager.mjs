#!/usr/bin/env node
/**
 * Launch the Memory Manager as a governed ACP session.
 *
 * Lifecycle: START_SESSION -> SEND_PROMPT -> CLOSE_SESSION.
 * CLOSE_SESSION is executed in finally so normal runs always reclaim governed terminals.
 *
 * Usage:
 *   node launch-memory-manager.mjs [--force] [--debug]
 *   just launch-memory-manager [--force] [--debug]
 *
 * --force launches regardless of staleness/activity thresholds.
 * --debug keeps the session open for manual observation and skips terminal reclaim.
 */

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  GOVERNANCE_RUNTIME_ROOT_ABS,
} from "../../../roles_shared/scripts/lib/runtime-paths.mjs";
import {
  openGovernanceMemoryDb,
  closeDb,
} from "../../../roles_shared/scripts/memory/governance-memory-lib.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const force = process.argv.includes("--force");
const debugMode = process.argv.includes("--debug");
const SESSION_CONTROL_SCRIPT = path.resolve(__dirname, "../../orchestrator/scripts/session-control-command.mjs");
const WP_ID = "WP-MEMORY-HYGIENE";
const ROLE = "MEMORY_MANAGER";
const REPORT_PATH = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "MEMORY_HYGIENE_REPORT.md");
const LAST_RUN_PATH = path.join(GOVERNANCE_RUNTIME_ROOT_ABS, "roles_shared", "MEMORY_MANAGER_LAST_RUN.json");
const STALENESS_MS = 24 * 60 * 60 * 1000;
const ACTIVITY_THRESHOLD = 10;

function log(message) {
  console.log(`[memory-manager] ${message}`);
}

function fail(message, details = "") {
  console.error(`[memory-manager] ${message}${details ? `\n${details}` : ""}`);
  process.exit(1);
}

function currentRunState() {
  try {
    if (!fs.existsSync(LAST_RUN_PATH)) return null;
    const raw = fs.readFileSync(LAST_RUN_PATH, "utf8");
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function setRunState(state) {
  fs.mkdirSync(path.dirname(LAST_RUN_PATH), { recursive: true });
  const payload = {
    ...state,
    run_at: new Date().toISOString(),
  };
  fs.writeFileSync(LAST_RUN_PATH, JSON.stringify(payload, null, 2), "utf8");
}

function countEntriesSince(lastRunAt) {
  let dbState;
  try {
    const { db } = openGovernanceMemoryDb();
    dbState = db;
    const row = db.prepare(
      "SELECT COUNT(*) as cnt FROM memory_index WHERE created_at > ?",
    ).get(lastRunAt || "1970-01-01") || {};
    return Number(row?.cnt || 0);
  } catch (error) {
    log(`Could not evaluate staleness activity gate: ${String(error?.message || error || "").slice(0, 200)}. Running for safety.`);
    return Number.MAX_SAFE_INTEGER;
  } finally {
    if (dbState) closeDb(dbState);
  }
}

function shouldRun() {
  if (force) {
    return {
      run: true,
      reason: "force mode enabled",
      newEntries: null,
    };
  }

  const lastRun = currentRunState();
  const lastRunAt = String(lastRun?.run_at || "").trim();
  if (!lastRunAt) {
    return {
      run: true,
      reason: "no previous run state found",
      newEntries: null,
    };
  }

  const lastRunTs = Date.parse(lastRunAt);
  if (Number.isNaN(lastRunTs)) {
    return {
      run: true,
      reason: `invalid run_at in ${LAST_RUN_PATH}; rerunning`,
      newEntries: null,
    };
  }

  const sinceMs = Date.now() - lastRunTs;
  if (sinceMs < STALENESS_MS) {
    return {
      run: false,
      reason: `last successful run was ${Math.round(sinceMs / (60 * 1000))} minutes ago`,
      newEntries: 0,
    };
  }

  const newEntries = countEntriesSince(lastRunAt);
  if (newEntries < ACTIVITY_THRESHOLD) {
    return {
      run: false,
      reason: `only ${newEntries} new entries since last run (threshold ${ACTIVITY_THRESHOLD})`,
      newEntries,
    };
  }

  return {
    run: true,
    reason: `${newEntries} new entries since last run`,
    newEntries,
  };
}

function sessionControl(commandKind, prompt = "", model = "PRIMARY") {
  const args = [
    SESSION_CONTROL_SCRIPT,
    commandKind,
    ROLE,
    WP_ID,
    prompt,
    model,
  ];
  const env = {
    ...process.env,
    ...(debugMode ? { HANDSHAKE_SESSION_CONTROL_DEBUG: "1" } : {}),
  };

  const result = spawnSync(process.execPath, args, {
    encoding: "utf8",
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const output = `${result.stdout || ""}${result.stderr || ""}`;
  if (output) {
    const trimmed = output.trim();
    if (trimmed) {
      log(trimmed.length <= 4000 ? trimmed : `${trimmed.slice(0, 4000)}...`);
    }
  }
  return {
    code: Number.isFinite(result.status) ? result.status : 1,
    commandKind,
    output,
  };
}

function buildPrompt(gate) {
  let previousReport = "";
  if (fs.existsSync(REPORT_PATH)) {
    try {
      previousReport = fs.readFileSync(REPORT_PATH, "utf8");
    } catch {
      previousReport = "";
    }
  }

  const timestamp = new Date().toISOString();
  return [
    "Run a governance memory hygiene pass.",
    "1) execute a full, read-only check for memory integrity and retention state.",
    "2) identify stale entries and summarize the highest-value memories added since the last checkpoint.",
    "3) report actionable cleanup recommendations.",
    "4) write a short markdown summary to:",
    REPORT_PATH,
    "",
    `Timestamp: ${timestamp}`,
    `Gate reason: ${gate.reason}`,
    gate.newEntries != null ? `New entries since last run: ${gate.newEntries}` : "New entries since last run: unknown",
    previousReport ? "\nPrior summary (first 8 lines):" : "",
    ...(previousReport ? previousReport.split(/\r?\n/).slice(0, 8) : []),
  ].filter(Boolean).join("\n");
}

async function main() {
  const gate = shouldRun();
  if (!gate.run) {
    log(`Skipping run: ${gate.reason}`);
    setRunState({
      status: "skipped",
      reason: gate.reason,
      debug: debugMode,
      force,
      run_after: new Date(Date.now() + STALENESS_MS).toISOString(),
      newEntries: gate.newEntries || 0,
    });
    return;
  }

  let closed = false;
  let promptResultCode = 1;
  let sendError = "";
  let startError = "";
  let closeOutput = "";
  let startResultCode = 1;
  const hygienePrompt = buildPrompt(gate);

  try {
    const startResult = sessionControl("START_SESSION", "", "PRIMARY");
    startResultCode = startResult.code;
    if (startResult.code !== 0) {
      startError = startResult.output;
    } else {
      const sendResult = sessionControl("SEND_PROMPT", hygienePrompt, "PRIMARY");
      promptResultCode = sendResult.code;
      if (sendResult.code === 0) {
        log("Memory manager hygiene prompt dispatched.");
      } else {
        sendError = sendResult.output;
      }
    }
  } finally {
    if (debugMode) {
      log("Debug mode enabled; skipping CLOSE_SESSION so session can be inspected manually.");
      setRunState({
        status: startResultCode !== 0 || promptResultCode !== 0 ? "debug_failed" : "debug",
        reason: gate.reason,
        debug: true,
        force,
        newEntries: gate.newEntries,
        startError: startError || "",
        sendError: sendError || "",
        promptResultCode,
      });
      return;
    }

    const closeResult = sessionControl("CLOSE_SESSION");
    closeOutput = closeResult.output || "";
    if (closeResult.code === 0) {
      closed = true;
      log("Memory manager session closed.");
    } else {
      log(`CLOSE_SESSION completed with status ${closeResult.code}. Terminal may require manual reclaim.`);
      if (closeOutput) {
        log(`CLOSE_SESSION output: ${closeOutput.slice(0, 300)}`);
      }
    }

  }

  if (startResultCode !== 0) {
    setRunState({
      status: "failed",
      reason: gate.reason,
      debug: false,
      force,
      newEntries: gate.newEntries,
      startError: startError || "",
      sendError: sendError || "",
      promptResultCode,
      closeOutput,
    });
    fail(`START_SESSION failed: ${startResultCode}`, (startError || "").slice(0, 200));
  }

  if (!closed) {
    setRunState({
      status: "failed",
      reason: gate.reason,
      debug: false,
      force,
      newEntries: gate.newEntries,
      startError: startError || "",
      sendError: sendError || "",
      promptResultCode,
      closeOutput,
    });
    fail("Could not close memory manager session cleanly; check session-control output above.");
  }

  if (promptResultCode !== 0) {
    setRunState({
      status: "failed",
      reason: gate.reason,
      debug: false,
      force,
      newEntries: gate.newEntries,
      startError: startError || "",
      sendError: sendError || "",
      promptResultCode,
      closeOutput,
    });
    fail(`SEND_PROMPT failed: ${promptResultCode}`, (sendError || "").slice(0, 200));
  }

  setRunState({
    status: "success",
    reason: gate.reason,
    debug: false,
    force,
    newEntries: gate.newEntries,
    startError: "",
    sendError: "",
    promptResultCode,
    closeOutput,
  });
}

main().catch((error) => {
  fail(`Unhandled exception: ${String(error?.message || error)}`);
});
