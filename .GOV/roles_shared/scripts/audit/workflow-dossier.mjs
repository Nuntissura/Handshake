#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import {
  loadSessionControlRequests,
  loadSessionControlResults,
  loadSessionRegistry,
} from "../session/session-registry-lib.mjs";
import {
  SESSION_CONTROL_BROKER_STATE_FILE,
} from "../session/session-policy.mjs";
import {
  buildWorkflowDossierIdleMetrics,
  buildWpMetrics,
  buildWpTimelineEntries,
  buildWpTimelineSpans,
  buildWpTimelineSummary,
  parseThreadEntriesText,
} from "../session/wp-timeline-lib.mjs";
import { readVerdictSettlementTruth } from "../lib/merge-progression-truth-lib.mjs";
import {
  NOTIFICATION_CURSOR_FILE_NAME,
  NOTIFICATIONS_FILE_NAME,
  parseJsonFile,
  parseJsonlFile,
} from "../lib/wp-communications-lib.mjs";
import { readExecutionProjectionView } from "../lib/wp-execution-state-lib.mjs";
import { resolveWorkPacketPath } from "../lib/runtime-paths.mjs";
import { checkAllNotifications } from "../wp/wp-check-notifications.mjs";
import {
  activeRunsForSession,
  buildSessionTelemetry,
  formatPushAlertInline,
  formatSessionRunTelemetryInline,
  formatSessionStepTelemetryInline,
  selectLatestPushAlert,
} from "../session/session-telemetry-lib.mjs";
import {
  appendWorkflowDossierEntry,
  evaluateWorkflowDossierJudgment,
  formatRepomemDossierEntry,
  formatRepomemDossierSnapshotEntry,
  formatWorkflowDossierTimestamp,
  normalizePath,
  normalizeWorkflowDossierSection,
  selectRepomemEntriesForWorkflowDossier,
  resolveWorkflowDossierPath,
} from "./workflow-dossier-lib.mjs";
import {
  openGovernanceMemoryDb,
  closeDb,
} from "../memory/governance-memory-lib.mjs";
import { evaluateWpRepomemCoverage } from "../memory/repomem-coverage-lib.mjs";
import { evaluateWpTokenBudget } from "../session/wp-token-budget-lib.mjs";
import { readWpTokenUsageLedger } from "../session/wp-token-usage-lib.mjs";
import {
  checkDetailsLogPath,
  readCheckDetails,
} from "../lib/check-result-lib.mjs";
import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("workflow-dossier.mjs", { role: "SHARED" });

function fail(message) {
  failWithMemory("workflow-dossier.mjs", message, { role: "SHARED" });
}

function repoRoot() {
  return execFileSync("git", ["rev-parse", "--show-toplevel"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function readBrokerState(rootDir) {
  const brokerPath = path.resolve(rootDir, SESSION_CONTROL_BROKER_STATE_FILE);
  const broker = fs.existsSync(brokerPath) ? parseJsonFile(brokerPath) : null;
  return {
    updatedAt: broker?.updated_at || "",
    activeRuns: Array.isArray(broker?.active_runs) ? broker.active_runs : [],
  };
}

function readRelayWatchdogSnapshot(rootDir, wpId) {
  const watchdogPath = path.resolve(rootDir, ".GOV", "roles", "orchestrator", "scripts", "wp-relay-watchdog.mjs");
  if (!fs.existsSync(watchdogPath)) return null;
  try {
    const output = execFileSync(process.execPath, [watchdogPath, wpId, "--observe-only", "--json"], {
      cwd: rootDir,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    const lines = String(output || "").split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
    if (lines.length === 0) return null;
    return JSON.parse(lines.at(-1));
  } catch {
    return null;
  }
}

function summarizeNotifications(wpId, commDir) {
  const results = checkAllNotifications({ wpId });
  const checks = Object.values(results);
  return {
    totalPending: checks.reduce((sum, entry) => sum + Number(entry.pendingCount || 0), 0),
    notificationsPath: normalizePath(path.join(commDir, NOTIFICATIONS_FILE_NAME)),
    cursorPath: normalizePath(path.join(commDir, NOTIFICATION_CURSOR_FILE_NAME)),
  };
}

function toDate(value) {
  if (!value) return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

function latestDate(values) {
  let latest = null;
  for (const value of values) {
    const date = toDate(value);
    if (!date) continue;
    if (!latest || date.getTime() > latest.getTime()) {
      latest = date;
    }
  }
  return latest;
}

function formatIdleMinutes(date, now = new Date()) {
  if (!date) return "N/A";
  return String(Math.max(0, Math.round((now.getTime() - date.getTime()) / 60000)));
}

function formatDurationCompact(value) {
  if (!Number.isFinite(value)) return "N/A";
  const totalSeconds = Math.max(0, Math.round(Number(value) / 1000));
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const totalMinutes = Math.round(totalSeconds / 60);
  if (totalMinutes < 60) return `${totalMinutes}m`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return minutes > 0 ? `${hours}h${minutes}m` : `${hours}h`;
}

function summarizeSessionLaneTelemetry(rootDir, session, { now, activeRuns, notifications } = {}) {
  const role = String(session?.role || "ROLE").trim().toUpperCase() || "ROLE";
  const telemetry = buildSessionTelemetry({
    session,
    activeRuns: activeRunsForSession(session, activeRuns),
    repoRoot: rootDir,
    now,
  });
  const latestPushAlert = selectLatestPushAlert(notifications, {
    targetRole: role,
    targetSession: String(session?.session_thread_id || "").trim(),
  });
  return [
    `${role}{${formatSessionRunTelemetryInline(telemetry.run)};${formatSessionStepTelemetryInline(telemetry.step)}}`,
    latestPushAlert ? `${role}{${formatPushAlertInline(latestPushAlert)}}` : "",
  ].filter(Boolean).join(";");
}

function formatInlineScalar(value, fallback = "none") {
  if (Array.isArray(value)) {
    return value.length > 0 ? value.join(",") : fallback;
  }
  const normalized = String(value ?? "").trim();
  return normalized ? normalized : fallback;
}

function formatLedgerGroup(label, fields = []) {
  const parts = fields.map(([key, value]) => `${key}=${formatInlineScalar(value)}`);
  return `${label}{${parts.join("; ") || "none"}}`;
}

function summarizeTokenTelemetry(tokenLedger = {}, tokenBudget = {}) {
  const grossInput = Number(tokenLedger?.summary?.usage_totals?.input_tokens || 0);
  const cachedInput = Number(tokenLedger?.summary?.usage_totals?.cached_input_tokens || 0);
  const freshInput = Math.max(0, grossInput - cachedInput);
  const output = Number(tokenLedger?.summary?.usage_totals?.output_tokens || 0);
  const turns = Number(tokenLedger?.summary?.turn_count || 0);
  const commands = Number(tokenLedger?.summary?.command_count || 0);
  return formatLedgerGroup("tokens", [
    ["policy", tokenBudget?.policy_id || "<missing>"],
    ["mode", tokenBudget?.enforcement_mode || "<missing>"],
    ["status", tokenBudget?.status || "<missing>"],
    [
      "ledger",
      `${formatInlineScalar(tokenLedger?.ledger_health?.status, "<missing>")}/${formatInlineScalar(tokenLedger?.ledger_health?.severity, "<missing>")}`,
    ],
    ["gross_in", fmtTokens(grossInput)],
    ["fresh_in", fmtTokens(freshInput)],
    ["cached_in", fmtTokens(cachedInput)],
    ["out", fmtTokens(output)],
    ["turns", String(turns)],
    ["commands", String(commands)],
  ]);
}

function formatCheckDetailsInline(details = {}) {
  const text = JSON.stringify(details || {});
  if (text.length <= 2400) return text;
  return `${text.slice(0, 2397)}...`;
}

function latestCheckDetailDossierLines({ wpId = "", limit = 5 } = {}) {
  const rows = readCheckDetails({ wpId, limit });
  return rows.map((row) => {
    const timestamp = row.timestamp || "UNKNOWN";
    const check = row.check || "unknown-check";
    const verdict = row.verdict || "UNKNOWN";
    const summary = row.summary || "<no summary>";
    const entryId = row.entry_id || "<no-entry-id>";
    return `- [${timestamp}] [CHECK_DETAIL] [${check}] ${verdict} | ${summary} | entry=${entryId} | details=${formatCheckDetailsInline(row.details || {})}`;
  });
}

function usage() {
  fail("Usage: node .GOV/roles_shared/scripts/audit/workflow-dossier.mjs <init|note|sync|inject-repomem|autofill-costs|judgment-check> WP-{ID} [args]");
}

function parseArgs(argv) {
  const args = [...argv];
  const command = String(args.shift() || "").trim().toLowerCase();
  if (!command) usage();
  const wpId = String(args.shift() || "").trim();
  if (!wpId || !wpId.startsWith("WP-")) {
    fail("WP_ID must start with WP-");
  }

  const options = {
    command,
    wpId,
    output: "",
    autoOutput: false,
    force: false,
    sessionIntention: "",
    section: "",
    summary: "",
    role: "ORCHESTRATOR",
    tag: "",
    surface: "",
    file: "",
    time: "",
    terminalVerdict: "",
  };

  if (command === "note") {
    options.section = String(args.shift() || "").trim();
    options.summary = String(args.shift() || "").trim();
    if (!options.section || !options.summary) {
      fail("Usage: node .GOV/roles_shared/scripts/audit/workflow-dossier.mjs note WP-{ID} <EXECUTION|GOV_CHANGE|CONCERN|FINDING> \"<summary>\" [--role ROLE] [--tag TAG] [--surface SURFACE] [--file file.md] [--time \"YYYY-MM-DD HH:MM:SS Europe/Brussels\"]");
    }
  }

  while (args.length > 0) {
    const token = String(args.shift() || "").trim();
    if (!token) continue;
    if (command === "init" && token === "--output") {
      options.output = String(args.shift() || "").trim();
      continue;
    }
    if (command === "init" && token === "--auto-output") {
      options.autoOutput = true;
      continue;
    }
    if (command === "init" && token === "--force") {
      options.force = true;
      continue;
    }
    if (command === "init" && token === "--session-intention") {
      options.sessionIntention = String(args.shift() || "").trim();
      continue;
    }
    if ((command === "note" || command === "sync") && token === "--role") {
      options.role = String(args.shift() || "").trim() || "ORCHESTRATOR";
      continue;
    }
    if ((command === "note" || command === "sync") && token === "--tag") {
      options.tag = String(args.shift() || "").trim();
      continue;
    }
    if ((command === "note" || command === "sync") && token === "--surface") {
      options.surface = String(args.shift() || "").trim();
      continue;
    }
    if ((command === "note" || command === "sync" || command === "inject-repomem" || command === "autofill-costs" || command === "judgment-check") && token === "--file") {
      options.file = String(args.shift() || "").trim();
      continue;
    }
    if (command === "judgment-check" && token === "--terminal-verdict") {
      options.terminalVerdict = String(args.shift() || "").trim().toUpperCase();
      continue;
    }
    // RGF-186: scope classification for findings/concerns
    if (command === "note" && token === "--scope-type") {
      options.scopeType = String(args.shift() || "").trim().toUpperCase();
      continue;
    }
    if (command === "note" && token === "--time") {
      options.time = String(args.shift() || "").trim();
      continue;
    }
    fail(`Unknown argument: ${token}`);
  }

  if (!["init", "note", "sync", "inject-repomem", "autofill-costs", "judgment-check"].includes(command)) {
    usage();
  }
  if (command === "init" && options.output && options.autoOutput) {
    fail("Use either --output or --auto-output, not both.");
  }

  return options;
}

// RGF-186: valid scope-type values for typed product-vs-governance classification.
// .GOV = repo-governance paths, PRODUCT = product code, PRODUCT_GOV_BEHAVIOR = product code
// implementing governance behavior, PACKET_DRIFT = signed-scope evidence drift.
const VALID_SCOPE_TYPES = new Set([".GOV", "PRODUCT", "PRODUCT_GOV_BEHAVIOR", "PACKET_DRIFT"]);

function buildNoteLine(options) {
  const section = normalizeWorkflowDossierSection(options.section);
  if (!section) {
    fail(`Unknown workflow dossier section: ${options.section}`);
  }
  const timestamp = formatWorkflowDossierTimestamp(options.time || new Date());
  const role = String(options.role || "ORCHESTRATOR").trim().toUpperCase();
  const tag = String(options.tag || "").trim().toUpperCase();
  const surface = String(options.surface || "").trim() || "MANUAL";
  const summary = String(options.summary || "").trim();

  // RGF-186: optional scope-type classification for CONCERN and FINDING entries.
  const rawScopeType = String(options.scopeType || "").trim().toUpperCase();
  const scopeType = VALID_SCOPE_TYPES.has(rawScopeType) ? rawScopeType : "";
  const scopeTag = scopeType ? ` {${scopeType}}` : "";

  if (section === "EXECUTION" || section === "IDLE") {
    return {
      section,
      line: `- [${timestamp}] [${role}] [${tag || "NOTE"}] [${surface}] ${summary}`,
    };
  }
  if (section === "GOV_CHANGE") {
    return {
      section,
      line: `- [${timestamp}] [${role}] [${tag || "PATCH"}] ${surface} :: ${summary}`,
    };
  }
  if (section === "CONCERN") {
    return {
      section,
      line: `- [${timestamp}] [${role}] [${tag || "CONCERN"}]${scopeTag} ${summary}`,
    };
  }
  return {
    section,
    line: `- [${timestamp}] [${role}] [${tag || "GENERAL"}]${scopeTag} ${summary}`,
  };
}

function runInit(rootDir, options) {
  const generatorPath = path.resolve(rootDir, ".GOV", "roles_shared", "scripts", "audit", "generate-post-run-audit-skeleton.mjs");
  const generatorArgs = [generatorPath, options.wpId, "--mode", "live"];
  if (options.output) {
    generatorArgs.push("--output", options.output);
  } else {
    generatorArgs.push("--auto-output");
  }
  if (options.force) {
    generatorArgs.push("--force");
  }
  if (options.sessionIntention) {
    generatorArgs.push("--session-intention", options.sessionIntention);
  }
  const result = execFileSync(process.execPath, generatorArgs, {
    cwd: rootDir,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  }).trim();
  if (result) {
    console.log(result);
  }
}

function runNote(rootDir, options) {
  const { section, line } = buildNoteLine(options);
  const role = String(options.role || "ORCHESTRATOR").trim().toUpperCase();
  const targetSection = role === "ORCHESTRATOR"
    ? "ORCHESTRATOR_DIAGNOSTIC"
    : section;
  const insertMode = role === "ORCHESTRATOR"
    ? "section-prepend"
    : "section-append";
  const dossierPath = appendWorkflowDossierEntry({
    repoRoot: rootDir,
    wpId: options.wpId,
    filePath: options.file,
    section: targetSection,
    line,
    insertMode,
  });
  if (!dossierPath) {
    fail(`No open workflow dossier found for ${options.wpId}. Run \`just workflow-dossier-init ${options.wpId}\` first or pass --file.`);
  }
  console.log(normalizePath(path.relative(rootDir, dossierPath)) || normalizePath(dossierPath));
}

function runSync(rootDir, options) {
  const nowMs = Date.now();
  const now = new Date(nowMs);
  const resolvedPacket = resolveWorkPacketPath(options.wpId);
  if (!resolvedPacket?.packetAbsPath || !fs.existsSync(resolvedPacket.packetAbsPath)) {
    fail(`Packet not found for ${options.wpId}`);
  }

  const packetText = fs.readFileSync(resolvedPacket.packetAbsPath, "utf8");
  const runtimePath = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsPath = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const threadPath = parseSingleField(packetText, "WP_THREAD_FILE");
  const commDir = parseSingleField(packetText, "WP_COMMUNICATION_DIR");
  const rawRuntimeStatus = runtimePath && fs.existsSync(path.resolve(rootDir, runtimePath))
    ? parseJsonFile(path.resolve(rootDir, runtimePath))
    : {};
  const runtimeProjection = readExecutionProjectionView(rawRuntimeStatus);
  const runtime = runtimeProjection.runtime;
  const receipts = receiptsPath && fs.existsSync(path.resolve(rootDir, receiptsPath))
    ? parseJsonlFile(path.resolve(rootDir, receiptsPath))
    : [];
  const threadEntries = threadPath && fs.existsSync(path.resolve(rootDir, threadPath))
    ? parseThreadEntriesText(fs.readFileSync(path.resolve(rootDir, threadPath), "utf8"))
    : [];
  const notificationsPath = commDir ? path.resolve(rootDir, commDir, NOTIFICATIONS_FILE_NAME) : "";
  const notifications = notificationsPath && fs.existsSync(notificationsPath)
    ? parseJsonlFile(notificationsPath)
    : [];
  const notificationSummary = summarizeNotifications(options.wpId, commDir);
  const { registry } = loadSessionRegistry(rootDir);
  const sessions = (registry.sessions || []).filter((session) => String(session.wp_id || "").trim() === options.wpId);
  const { requests } = loadSessionControlRequests(rootDir);
  const { results } = loadSessionControlResults(rootDir);
  const controlRequests = requests.filter((entry) => String(entry.wp_id || "").trim() === options.wpId);
  const controlResults = results.filter((entry) => String(entry.wp_id || "").trim() === options.wpId);
  const tokenUsage = readWpTokenUsageLedger(rootDir, options.wpId);
  const tokenLedger = tokenUsage?.ledger || {};
  const tokenBudget = evaluateWpTokenBudget(tokenLedger);
  const repomemCoverage = evaluateWpRepomemCoverage({
    repoRoot: rootDir,
    wpId: options.wpId,
    packetContent: packetText,
    receipts,
    threadEntries,
    sessions,
    controlRequests,
    controlResults,
  });
  const brokerSummary = readBrokerState(rootDir);
  const latestControlResult = controlResults.at(-1) || {};
  const latestReceipt = receipts.at(-1) || {};
  const relayWatchdog = readRelayWatchdogSnapshot(rootDir, options.wpId);
  const relayLaneVerdict = relayWatchdog?.laneVerdict || null;
  const workerInterruptBudget = relayWatchdog?.workerInterruptBudget || null;
  const sessionActivitySummary = sessions
    .map((session) => summarizeSessionLaneTelemetry(rootDir, session, {
      now,
      activeRuns: brokerSummary.activeRuns,
      notifications,
    }))
    .filter(Boolean)
    .join(",");
  const timelineEntries = buildWpTimelineEntries({
    threadEntries,
    receipts,
    notifications,
    controlRequests,
    controlResults,
  });
  const timelineSpans = buildWpTimelineSpans({
    receipts,
    controlRequests,
    controlResults,
  });
  const idleMetrics = buildWorkflowDossierIdleMetrics({
    entries: timelineEntries,
    spans: timelineSpans,
    receipts,
    notifications,
    controlRequests,
    controlResults,
    runtimeStatus: runtime,
    laneVerdict: relayLaneVerdict,
    pendingNotificationCount: notificationSummary.totalPending,
    now: nowMs,
  });
  const latestMechanicalEvent = latestDate([
    runtimeProjection.last_event_at,
    latestControlResult.completed_at,
    latestControlResult.created_at,
    latestControlResult.timestamp,
    latestReceipt.timestamp_utc,
    latestReceipt.created_at,
    brokerSummary.updatedAt,
  ]);

  const runtimeStatus = runtimeProjection.runtime_status || "NONE";
  const waitingOn = runtimeProjection.waiting_on || runtimeProjection.next_expected_actor || "NONE";
  const latestControlSummary = latestControlResult.command_kind
    ? `${latestControlResult.command_kind}/${latestControlResult.status || "UNKNOWN"}`
    : "NONE";
  const latestReceiptSummary = latestReceipt.receipt_kind
    ? `${latestReceipt.receipt_kind}@${latestReceipt.timestamp_utc || "NONE"}`
    : "NONE";
  const verdictSettlement = readVerdictSettlementTruth({
    packetText,
    runtimeStatus: rawRuntimeStatus,
  });
  const flowGraph = `BROKER(${brokerSummary.activeRuns.length} active) -> ${options.wpId} [${runtimeStatus} / waiting_on=${waitingOn}]`;
  const syncSummary = [
    formatLedgerGroup("counts", [
      ["sessions", String(sessions.length)],
      ["control", `${controlRequests.length}/${controlResults.length}`],
      ["receipts", String(receipts.length)],
      ["pending", String(notificationSummary.totalPending)],
    ]),
    formatLedgerGroup("latest", [
      ["control", latestControlSummary],
      ["receipt", latestReceiptSummary],
    ]),
    formatLedgerGroup("route", [
      ["run_step", sessionActivitySummary || "NONE"],
      ["push_alert", formatPushAlertInline(selectLatestPushAlert(notifications, { targetRole: "ORCHESTRATOR" }))],
      ["lane", relayLaneVerdict ? `${relayLaneVerdict.verdict}/${relayLaneVerdict.reasonCode}` : "NONE"],
    ]),
    formatLedgerGroup("settlement", [
      ["verdict", verdictSettlement.verdictOfRecord || "UNKNOWN"],
      ["state", verdictSettlement.settlementState],
      ["blockers", verdictSettlement.settlementBlockers.join(",") || "none"],
    ]),
    formatLedgerGroup("repomem", [
      ["state", repomemCoverage.state],
      ["roles", repomemCoverage.active_roles.join(",") || "none"],
      ["debt", repomemCoverage.debt_keys.join(",") || "none"],
    ]),
    summarizeTokenTelemetry(tokenLedger, tokenBudget),
    formatLedgerGroup("host", [
      ["load", "HEAVY_ASSUMED"],
      ["interrupt_budget", workerInterruptBudget ? `${workerInterruptBudget.currentCycle}/${workerInterruptBudget.maxCycle}` : "NONE"],
      ["idle", `${formatIdleMinutes(latestMechanicalEvent)}m`],
    ]),
  ].join(" | ");
  const idleThresholdLabel = formatDurationCompact(idleMetrics.idle_threshold_ms);
  const currentWait = idleMetrics.downtime_attribution?.current_wait || { bucket: "NONE", duration_ms: null, reason: "NONE" };
  const idleSummary = [
    formatLedgerGroup("latency", [
      [
        "review_rtt",
        `last=${formatDurationCompact(idleMetrics.review_response.latest_ms)},max=${formatDurationCompact(idleMetrics.review_response.max_ms)},open=${idleMetrics.review_response.open_count}`,
      ],
      [
        "pass_to_coder",
        `last=${formatDurationCompact(idleMetrics.validator_pass_to_next_coder_action.latest_ms)},max=${formatDurationCompact(idleMetrics.validator_pass_to_next_coder_action.max_ms)},waiting=${idleMetrics.validator_pass_to_next_coder_action.waiting_count}`,
      ],
    ]),
    formatLedgerGroup("idle", [
      ["current", formatDurationCompact(idleMetrics.current_idle_ms)],
      ["max_gap", formatDurationCompact(idleMetrics.max_idle_gap_ms)],
      ["gaps", `${idleThresholdLabel}:${idleMetrics.idle_gap_count}`],
    ]),
    formatLedgerGroup("wall_clock", [
      ["active", formatDurationCompact(idleMetrics.downtime_attribution.active_build_ms)],
      ["validator", formatDurationCompact(idleMetrics.downtime_attribution.validator_wait_ms)],
      ["route", formatDurationCompact(idleMetrics.downtime_attribution.route_wait_ms)],
      ["dependency", formatDurationCompact(idleMetrics.downtime_attribution.dependency_wait_ms)],
      ["human", formatDurationCompact(idleMetrics.downtime_attribution.human_wait_ms)],
      ["repair", formatDurationCompact(idleMetrics.downtime_attribution.repair_overhead_ms)],
    ]),
    formatLedgerGroup("current_wait", [
      ["bucket", currentWait.bucket],
      ["duration", formatDurationCompact(currentWait.duration_ms)],
      ["reason", currentWait.reason || "NONE"],
    ]),
    formatLedgerGroup("queue", [
      ["level", idleMetrics.queue_pressure.level],
      ["score", String(idleMetrics.queue_pressure.score)],
      ["pending", String(idleMetrics.queue_pressure.pending_notification_count)],
      ["open_reviews", String(idleMetrics.queue_pressure.open_review_count)],
      ["open_control", String(idleMetrics.queue_pressure.unresolved_control_count)],
    ]),
    formatLedgerGroup("drift", [
      ["dup_receipts", String(idleMetrics.drift_markers.duplicate_receipt_count)],
      ["open_reviews", String(idleMetrics.drift_markers.open_review_count)],
      ["open_control", String(idleMetrics.drift_markers.unresolved_control_count)],
    ]),
  ].join(" | ");
  const timestamp = formatWorkflowDossierTimestamp(new Date());
  const role = String(options.role || "ORCHESTRATOR").trim().toUpperCase();
  const tag = String(options.tag || "ACP_SYNC").trim().toUpperCase() || "ACP_SYNC";
  const surface = String(options.surface || "MECHANICAL").trim() || "MECHANICAL";
  const executionPayload = `[${role}] [${tag}] [${surface}] \`${flowGraph}\` | ${syncSummary}`;
  const idlePayload = `[${role}] [IDLE_LEDGER] [${surface}] \`${options.wpId}\` | ${idleSummary}`;

  const dossierPath = appendWorkflowDossierEntry({
    repoRoot: rootDir,
    wpId: options.wpId,
    filePath: options.file,
    section: "ACP_TRACE",
    line: `- [${timestamp}] ${executionPayload}`,
    dedupeSuffix: executionPayload,
    insertMode: "section-append",
  });
  if (!dossierPath) {
    fail(`No open workflow dossier found for ${options.wpId}. Run \`just workflow-dossier-init ${options.wpId}\` first or pass --file.`);
  }
  appendWorkflowDossierEntry({
    repoRoot: rootDir,
    wpId: options.wpId,
    filePath: options.file,
    section: "IDLE",
    line: `- [${timestamp}] ${idlePayload}`,
    dedupeSuffix: idlePayload,
    insertMode: "section-append",
  });
  const checkDetailsPath = checkDetailsLogPath({ wpId: options.wpId });
  const checkDetailLines = latestCheckDetailDossierLines({ wpId: options.wpId, limit: 5 });
  if (checkDetailLines.length > 0) {
    for (const line of checkDetailLines.reverse()) {
      appendWorkflowDossierEntry({
        repoRoot: rootDir,
        wpId: options.wpId,
        filePath: options.file,
        section: "ORCHESTRATOR_DIAGNOSTIC",
        line,
        dedupeSuffix: line.match(/entry=([^ ]+)/)?.[1] || line,
        insertMode: "section-prepend",
      });
    }
    appendWorkflowDossierEntry({
      repoRoot: rootDir,
      wpId: options.wpId,
      filePath: options.file,
      section: "ORCHESTRATOR_DIAGNOSTIC",
      line: `- [${timestamp}] [${role}] [CHECK_DETAILS] [${surface}] latest=${checkDetailLines.length} log=${normalizePath(path.relative(rootDir, checkDetailsPath)) || normalizePath(checkDetailsPath)}`,
      dedupeSuffix: `CHECK_DETAILS_LOG:${checkDetailLines.map((line) => line.match(/entry=([^ ]+)/)?.[1] || line).join(",")}`,
      insertMode: "section-prepend",
    });
  }
  console.log(normalizePath(path.relative(rootDir, dossierPath)) || normalizePath(dossierPath));
}

function runInjectRepomem(rootDir, options) {
  const dossierPath = resolveWorkflowDossierPath(rootDir, { wpId: options.wpId, filePath: options.file });
  if (!dossierPath) {
    fail(`No open workflow dossier found for ${options.wpId}. Run \`just workflow-dossier-init ${options.wpId}\` first or pass --file.`);
  }

  const dossierContent = fs.readFileSync(dossierPath, "utf8");

  // Parse OPENED_AT_UTC from dossier metadata.
  const openedMatch = dossierContent.match(/^-\s*OPENED_AT_UTC:\s*(.+)$/m);
  const openedAtUtc = openedMatch ? openedMatch[1].trim() : "1970-01-01T00:00:00.000Z";
  const openedAtSource = openedMatch ? "OPENED_AT_UTC" : "MISSING_OPENED_AT_UTC_FALLBACK";

  // Open governance memory DB and query conversation_log.
  const { db } = openGovernanceMemoryDb();

  let rawEntries;
  try {
    const sql = `SELECT * FROM conversation_log
      WHERE (wp_id = ? OR wp_id = '')
        AND timestamp_utc >= ?
      ORDER BY timestamp_utc ASC`;
    rawEntries = db.prepare(sql).all(options.wpId, openedAtUtc);
  } finally {
    closeDb(db);
  }

  const entries = selectRepomemEntriesForWorkflowDossier(rawEntries, { wpId: options.wpId });
  if (!entries || entries.length === 0) {
    console.log(`[workflow-dossier inject-repomem] No WP-bound repomem entries for ${options.wpId} since ${openedAtUtc} (${openedAtSource}). 0 appended (${rawEntries?.length || 0} raw matched).`);
    return;
  }

  // Read current dossier content for idempotence checks.
  const currentContent = fs.readFileSync(dossierPath, "utf8");
  let appended = 0;

  for (const entry of entries) {
    const formatted = formatRepomemDossierSnapshotEntry(entry) || formatRepomemDossierEntry(entry);
    if (!formatted) continue;

    // Idempotence: skip if a line with this session_id and timestamp already exists.
    const dedupeMarker = `[${formatted.tag}] [GOVERNANCE_MEMORY] [${formatted.sessionId}]`;
    if (currentContent.includes(dedupeMarker) && currentContent.includes(formatted.timestamp)) {
      continue;
    }

    appendWorkflowDossierEntry({
      repoRoot: rootDir,
      wpId: options.wpId,
      filePath: options.file,
      section: formatted.section,
      line: formatted.line,
      insertMode: "section-append",
    });
    appended++;
  }

  console.log(`[workflow-dossier inject-repomem] ${appended} entries appended to ${normalizePath(path.relative(rootDir, dossierPath))} (${entries.length} WP-bound, ${rawEntries?.length || 0} raw matched since ${openedAtUtc} via ${openedAtSource}, ${entries.length - appended} skipped as duplicates).`);
}

// RGF-187: Autofill Cost Attribution and Comparison Table from live timeline data.
function loadWpTimelineData(rootDir, wpId) {
  const resolvedPacket = resolveWorkPacketPath(wpId);
  if (!resolvedPacket?.packetAbsPath || !fs.existsSync(resolvedPacket.packetAbsPath)) {
    fail(`Packet not found for ${wpId}`);
  }
  const packetText = fs.readFileSync(resolvedPacket.packetAbsPath, "utf8");
  const runtimePath = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsPath = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const threadPath = parseSingleField(packetText, "WP_THREAD_FILE");
  const commDir = parseSingleField(packetText, "WP_COMMUNICATION_DIR");
  const runtimeProjection = runtimePath && fs.existsSync(path.resolve(rootDir, runtimePath))
    ? readExecutionProjectionView(parseJsonFile(path.resolve(rootDir, runtimePath)))
    : readExecutionProjectionView({});
  const runtime = runtimeProjection.runtime;
  const receipts = receiptsPath && fs.existsSync(path.resolve(rootDir, receiptsPath))
    ? parseJsonlFile(path.resolve(rootDir, receiptsPath)) : [];
  const threadEntries = threadPath && fs.existsSync(path.resolve(rootDir, threadPath))
    ? parseThreadEntriesText(fs.readFileSync(path.resolve(rootDir, threadPath), "utf8")) : [];
  const notificationsPath = commDir ? path.resolve(rootDir, commDir, NOTIFICATIONS_FILE_NAME) : "";
  const notifications = notificationsPath && fs.existsSync(notificationsPath)
    ? parseJsonlFile(notificationsPath) : [];
  const { requests } = loadSessionControlRequests(rootDir);
  const { results } = loadSessionControlResults(rootDir);
  const controlRequests = requests.filter((e) => String(e.wp_id || "").trim() === wpId);
  const controlResults = results.filter((e) => String(e.wp_id || "").trim() === wpId);
  const tokenUsage = readWpTokenUsageLedger(rootDir, wpId);
  const tokenLedger = tokenUsage?.ledger || {};
  const timelineEntries = buildWpTimelineEntries({ threadEntries, receipts, notifications, controlRequests, controlResults });
  const timelineSpans = buildWpTimelineSpans({ receipts, controlRequests, controlResults });
  const summary = buildWpTimelineSummary({
    wpId,
    packetPath: resolvedPacket.packetPath,
    runtimeStatus: runtime,
    receipts, notifications, controlRequests, controlResults,
    tokenLedger,
    entries: timelineEntries,
    spans: timelineSpans,
  });
  const tokenBudget = evaluateWpTokenBudget(tokenLedger);
  return { summary, receipts, controlResults, tokenLedger, tokenBudget };
}

function fmtMin(value) {
  if (!Number.isFinite(value) || value === 0) return "0";
  return String(Math.round(value * 10) / 10);
}

function fmtTokens(value) {
  if (!Number.isFinite(value) || value === 0) return "0";
  if (value >= 1000000) return `${Math.round(value / 100000) / 10}M`;
  if (value >= 1000) return `${Math.round(value / 100) / 10}K`;
  return String(Math.round(value));
}

function buildRoleTokenDetailTable(tokenLedger = {}, tokenBudget = {}) {
  const roleTotals = tokenLedger?.role_totals || {};
  const budgetRoles = tokenBudget?.roles || {};
  const roleNames = Array.from(new Set([
    ...Object.keys(roleTotals),
    ...Object.keys(budgetRoles),
  ])).sort((left, right) => left.localeCompare(right));
  const rows = [
    `| Role | Commands | Turns | Gross In | Fresh In | Cached In | Out | Status |`,
    `|---|---|---|---|---|---|---|---|`,
  ];
  if (roleNames.length === 0) {
    rows.push(`| NONE | 0 | 0 | 0 | 0 | 0 | 0 | NO_OUTPUTS |`);
  }
  for (const roleName of roleNames) {
    const totals = roleTotals[roleName] || {};
    const usageTotals = totals?.usage_totals || {};
    const grossInput = Number(usageTotals.input_tokens || 0);
    const cachedInput = Number(usageTotals.cached_input_tokens || 0);
    const freshInput = Math.max(0, grossInput - cachedInput);
    const output = Number(usageTotals.output_tokens || 0);
    rows.push(
      `| ${roleName} | ${Number(totals.command_count || 0)} | ${Number(totals.turn_count || 0)} | ${fmtTokens(grossInput)} | ${fmtTokens(freshInput)} | ${fmtTokens(cachedInput)} | ${fmtTokens(output)} | ${budgetRoles[roleName]?.status || "N/A"} |`,
    );
  }
  rows.push(
    `| TOTAL | ${Number(tokenLedger?.summary?.command_count || 0)} | ${Number(tokenLedger?.summary?.turn_count || 0)} | ${fmtTokens(tokenLedger?.summary?.usage_totals?.input_tokens || 0)} | ${fmtTokens(Math.max(0, Number(tokenLedger?.summary?.usage_totals?.input_tokens || 0) - Number(tokenLedger?.summary?.usage_totals?.cached_input_tokens || 0)))} | ${fmtTokens(tokenLedger?.summary?.usage_totals?.cached_input_tokens || 0)} | ${fmtTokens(tokenLedger?.summary?.usage_totals?.output_tokens || 0)} | ${tokenBudget?.status || "NO_OUTPUTS"} |`,
  );
  return rows.join("\n");
}

function buildCostTable(metrics, tokenLedger = {}, tokenBudget = {}) {
  const dt = metrics;
  const totalMin = dt.wall_clock_minutes || 0;
  const activeMin = dt.product_active_minutes || 0;
  const repairMin = dt.repair_minutes || 0;
  const valMin = dt.validator_wait_minutes || 0;
  const routeMin = dt.route_wait_minutes || 0;
  const coderMin = dt.coder_wait_minutes || 0;
  const pollingMin = routeMin + coderMin;
  const fixMin = repairMin;
  const tokGrossIn = dt.token_gross_input_total ?? dt.token_input_total ?? 0;
  const tokFreshIn = dt.token_fresh_input_total ?? Math.max(0, Number(dt.token_input_total || 0) - Number(dt.token_cached_input_total || 0));
  const tokCachedIn = dt.token_cached_input_total || 0;
  const tokIn = tokGrossIn;
  const tokOut = dt.token_output_total || 0;
  const turns = dt.token_turn_count || 0;
  const rows = [
    `| Phase | Time (min) | Tokens | Notes |`,
    `|---|---|---|---|`,
    `| Product active | ${fmtMin(activeMin)} | — | implementation + test |`,
    `| Validation | ${fmtMin(valMin)} | — | validator wait |`,
    `| Fix/Repair | ${fmtMin(fixMin)} | — | governance repair overhead |`,
    `| Routing/Waiting | ${fmtMin(pollingMin)} | — | route + coder wait |`,
    `| TOTAL | ${fmtMin(totalMin)} | ${fmtTokens(tokIn)} in / ${fmtTokens(tokOut)} out / ${turns} turns | gov overhead ratio: ${metrics.governance_overhead_ratio != null ? Math.round(metrics.governance_overhead_ratio * 100) + "%" : "N/A"} |`,
  ];
  const tokenRows = [
    `| Metric | Value | Notes |`,
    `|---|---|---|`,
    `| Policy | ${metrics.budget_policy_id || "N/A"} | token-cost diagnostic contract |`,
    `| Enforcement mode | ${metrics.budget_enforcement_mode || "N/A"} | cost overrun does not block WP completion |`,
    `| Budget status | ${metrics.budget_status || "N/A"} | ${formatInlineScalar(tokenBudget?.summary, "none")} |`,
    `| Ledger health | ${formatInlineScalar(metrics.ledger_health, "N/A")} / ${formatInlineScalar(metrics.ledger_health_severity, "N/A")} | ${formatInlineScalar(tokenLedger?.ledger_health?.summary, "none")} |`,
    `| Tokens in (gross) | ${fmtTokens(tokGrossIn)} | includes cached replay |`,
    `| Tokens in (fresh) | ${fmtTokens(tokFreshIn)} | new-context spend proxy |`,
    `| Tokens in (cached) | ${fmtTokens(tokCachedIn)} | replay / compaction signal |`,
    `| Tokens out | ${fmtTokens(tokOut)} | |`,
    `| Turns | ${turns} | |`,
    `| Token commands | ${metrics.token_command_count ?? 0} | settled commands with usage |`,
    `| Host stance | HEAVY_ASSUMED | shell/bundle timing is non-authoritative under load |`,
  ];
  return [
    "### 14.1 Time Attribution",
    "",
    ...rows,
    "",
    "### 14.2 Token Diagnostics",
    "",
    ...tokenRows,
    "",
    "### 14.3 Role Token Breakdown",
    "",
    buildRoleTokenDetailTable(tokenLedger, tokenBudget),
  ].join("\n");
}

function buildComparisonTable(metrics) {
  const rows = [
    `| Metric | This WP | Notes |`,
    `|---|---|---|`,
    `| Workflow lane | ${metrics.workflow_lane || "N/A"} | runtime: ${metrics.runtime_status || "N/A"} / ${metrics.current_phase || "N/A"} |`,
    `| Wall clock (min) | ${fmtMin(metrics.wall_clock_minutes)} | |`,
    `| Microtask count | ${metrics.mt_count ?? "N/A"} | |`,
    `| Fix cycles | ${metrics.fix_cycles ?? 0} | |`,
    `| Governed receipts | ${metrics.receipt_count ?? 0} | |`,
    `| ACP commands | ${metrics.acp_commands ?? 0} | failures: ${metrics.acp_failures ?? 0} |`,
    `| Session restarts | ${metrics.session_restarts ?? 0} | |`,
    `| Tokens in (gross) | ${fmtTokens(metrics.token_gross_input_total ?? metrics.token_input_total)} | includes cached replay |`,
    `| Tokens in (fresh) | ${fmtTokens(metrics.token_fresh_input_total)} | new-context spend proxy |`,
    `| Tokens in (cached) | ${fmtTokens(metrics.token_cached_input_total)} | replay / compaction signal |`,
    `| Tokens out | ${fmtTokens(metrics.token_output_total)} | |`,
    `| Turns | ${metrics.token_turn_count ?? 0} | |`,
    `| Token commands | ${metrics.token_command_count ?? 0} | |`,
    `| Budget status | ${metrics.budget_status || "N/A"} | ${metrics.budget_enforcement_mode || "N/A"} |`,
    `| Ledger health | ${formatInlineScalar(metrics.ledger_health, "N/A")} / ${formatInlineScalar(metrics.ledger_health_severity, "N/A")} | ${formatInlineScalar(metrics.ledger_health_policy_id, "N/A")} / ${formatInlineScalar(metrics.ledger_health_drift_class, "N/A")} |`,
  ];
  return rows.join("\n");
}

function replaceMarkdownSection(content, headerPattern, body) {
  const match = content.match(headerPattern);
  if (!match || match.index == null) {
    return { replaced: false, content };
  }
  const headerLineEnd = content.indexOf("\n", match.index);
  if (headerLineEnd < 0) {
    return { replaced: false, content };
  }
  const nextHeaderIndex = content.indexOf("\n## ", headerLineEnd + 1);
  const sectionEnd = nextHeaderIndex >= 0 ? nextHeaderIndex + 1 : content.length;
  const before = content.slice(0, headerLineEnd + 1);
  const after = content.slice(sectionEnd).replace(/^\n+/, "");
  return {
    replaced: true,
    content: `${before}\n${body.trimEnd()}\n\n${after}`,
  };
}

function runAutofillCosts(rootDir, options) {
  const dossierPath = resolveWorkflowDossierPath(rootDir, { wpId: options.wpId, filePath: options.file });
  if (!dossierPath) {
    fail(`No workflow dossier found for ${options.wpId}. Pass --file for closed dossiers.`);
  }

  const { summary, receipts, controlResults, tokenLedger, tokenBudget } = loadWpTimelineData(rootDir, options.wpId);
  const metrics = buildWpMetrics({ wpId: options.wpId, summary, receipts, controlResults });

  let content = fs.readFileSync(dossierPath, "utf8");
  let replaced = 0;
  let replacement = replaceMarkdownSection(
    content,
    /^## 14\. Cost Attribution$/m,
    buildCostTable(metrics, tokenLedger, tokenBudget),
  );
  if (replacement.replaced) {
    content = replacement.content;
    replaced++;
  }
  replacement = replaceMarkdownSection(
    content,
    /^## 15\. Comparison Table.*$/m,
    buildComparisonTable(metrics),
  );
  if (replacement.replaced) {
    content = replacement.content;
    replaced++;
  }

  // Replace section 14 — Cost Attribution
  const costSectionRe = /$^/;
  if (costSectionRe.test(content)) {
    content = content.replace(costSectionRe, `$1${buildCostTable(metrics)}\n`);
    replaced++;
  }

  // Replace section 15 — Comparison Table
  const compSectionRe = /$^/;
  if (compSectionRe.test(content)) {
    content = content.replace(compSectionRe, `$1${buildComparisonTable(metrics)}\n`);
    replaced++;
  }

  if (replaced > 0) {
    fs.writeFileSync(dossierPath, content, "utf8");
  }
  console.log(`[workflow-dossier autofill-costs] ${replaced} section(s) replaced in ${normalizePath(path.relative(rootDir, dossierPath))}`);
  if (replaced === 0) {
    console.log(`[workflow-dossier autofill-costs] WARNING: no placeholder sections found to replace.`);
  }
}

function runJudgmentCheck(rootDir, options) {
  const dossierPath = resolveWorkflowDossierPath(rootDir, { wpId: options.wpId, filePath: options.file });
  if (!dossierPath) {
    fail(`No workflow dossier found for ${options.wpId}. Pass --file for closed dossiers.`);
  }
  const content = fs.readFileSync(dossierPath, "utf8");
  const terminalVerdict = String(options.terminalVerdict || "").trim().toUpperCase();
  const result = evaluateWorkflowDossierJudgment({
    content,
    terminalTruth: {
      terminal: Boolean(terminalVerdict),
      verdict: terminalVerdict || "UNKNOWN",
    },
  });
  console.log(`[workflow-dossier judgment-check] ${result.ok ? "PASS" : "FAIL"}: ${result.summary}`);
  console.log(`[workflow-dossier judgment-check] file=${normalizePath(path.relative(rootDir, dossierPath)) || normalizePath(dossierPath)}`);
  for (const diagnostic of result.diagnostics || []) {
    console.log(`  - ${diagnostic.code}: line=${diagnostic.line} ${diagnostic.message}${diagnostic.evidence ? ` | ${diagnostic.evidence}` : ""}`);
  }
  if (!result.ok) process.exit(1);
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const rootDir = repoRoot();
  if (options.command === "init") {
    runInit(rootDir, options);
    return;
  }
  if (options.command === "note") {
    runNote(rootDir, options);
    return;
  }
  if (options.command === "inject-repomem") {
    runInjectRepomem(rootDir, options);
    return;
  }
  if (options.command === "autofill-costs") {
    runAutofillCosts(rootDir, options);
    return;
  }
  if (options.command === "judgment-check") {
    runJudgmentCheck(rootDir, options);
    return;
  }
  runSync(rootDir, options);
}

main();
