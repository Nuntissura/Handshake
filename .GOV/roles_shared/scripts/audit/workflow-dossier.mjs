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
import {
  inspectSessionOutputActivity,
  summarizeActivityEvent,
} from "../session/session-output-activity-lib.mjs";
import {
  NOTIFICATION_CURSOR_FILE_NAME,
  NOTIFICATIONS_FILE_NAME,
  parseJsonFile,
  parseJsonlFile,
} from "../lib/wp-communications-lib.mjs";
import { resolveWorkPacketPath } from "../lib/runtime-paths.mjs";
import { checkAllNotifications } from "../wp/wp-check-notifications.mjs";
import {
  appendWorkflowDossierEntry,
  formatWorkflowDossierTimestamp,
  normalizePath,
  normalizeWorkflowDossierSection,
  resolveWorkflowDossierPath,
} from "./workflow-dossier-lib.mjs";
import {
  openGovernanceMemoryDb,
  closeDb,
} from "../memory/governance-memory-lib.mjs";
import { readWpTokenUsageLedger } from "../session/wp-token-usage-lib.mjs";
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

function summarizeSessionLaneActivity(rootDir, session, nowMs) {
  const role = String(session?.role || "ROLE").trim().toUpperCase() || "ROLE";
  const runtimeState = String(session?.runtime_state || "NONE").trim().toUpperCase() || "NONE";
  const outputFile = String(session?.last_command_output_file || "").trim();
  if (!outputFile) {
    return `${role}:${runtimeState}:no_output`;
  }
  const activity = inspectSessionOutputActivity(path.resolve(rootDir, outputFile), { nowMs });
  if (activity.latestProgressEvent) {
    return `${role}:${runtimeState}:${summarizeActivityEvent(activity.latestProgressEvent, { nowMs })}`;
  }
  if (activity.latestAgentMessageEvent) {
    return `${role}:${runtimeState}:${summarizeActivityEvent(activity.latestAgentMessageEvent, { nowMs })}`;
  }
  if (Number.isInteger(activity.outputFileIdleSeconds)) {
    return `${role}:${runtimeState}:output@${formatDurationCompact(activity.outputFileIdleSeconds * 1000)}`;
  }
  return `${role}:${runtimeState}:no_activity`;
}

function usage() {
  fail("Usage: node .GOV/roles_shared/scripts/audit/workflow-dossier.mjs <init|note|sync|inject-repomem|autofill-costs> WP-{ID} [args]");
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
    if ((command === "note" || command === "sync" || command === "inject-repomem" || command === "autofill-costs") && token === "--file") {
      options.file = String(args.shift() || "").trim();
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

  if (!["init", "note", "sync", "inject-repomem", "autofill-costs"].includes(command)) {
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
  const dossierPath = appendWorkflowDossierEntry({
    repoRoot: rootDir,
    wpId: options.wpId,
    filePath: options.file,
    section,
    line,
  });
  if (!dossierPath) {
    fail(`No open workflow dossier found for ${options.wpId}. Run \`just workflow-dossier-init ${options.wpId}\` first or pass --file.`);
  }
  console.log(normalizePath(path.relative(rootDir, dossierPath)) || normalizePath(dossierPath));
}

function runSync(rootDir, options) {
  const nowMs = Date.now();
  const resolvedPacket = resolveWorkPacketPath(options.wpId);
  if (!resolvedPacket?.packetAbsPath || !fs.existsSync(resolvedPacket.packetAbsPath)) {
    fail(`Packet not found for ${options.wpId}`);
  }

  const packetText = fs.readFileSync(resolvedPacket.packetAbsPath, "utf8");
  const runtimePath = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsPath = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const threadPath = parseSingleField(packetText, "WP_THREAD_FILE");
  const commDir = parseSingleField(packetText, "WP_COMMUNICATION_DIR");
  const runtime = runtimePath && fs.existsSync(path.resolve(rootDir, runtimePath))
    ? parseJsonFile(path.resolve(rootDir, runtimePath))
    : {};
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
  const brokerSummary = readBrokerState(rootDir);
  const latestControlResult = controlResults.at(-1) || {};
  const latestReceipt = receipts.at(-1) || {};
  const relayWatchdog = readRelayWatchdogSnapshot(rootDir, options.wpId);
  const relayLaneVerdict = relayWatchdog?.laneVerdict || null;
  const workerInterruptBudget = relayWatchdog?.workerInterruptBudget || null;
  const sessionActivitySummary = sessions
    .map((session) => summarizeSessionLaneActivity(rootDir, session, nowMs))
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
    runtime?.last_event_at,
    latestControlResult.completed_at,
    latestControlResult.created_at,
    latestControlResult.timestamp,
    latestReceipt.timestamp_utc,
    latestReceipt.created_at,
    brokerSummary.updatedAt,
  ]);

  const runtimeStatus = String(runtime?.runtime_status || runtime?.status || "NONE").trim() || "NONE";
  const waitingOn = String(runtime?.waiting_on || runtime?.next_actor || "NONE").trim() || "NONE";
  const latestControlSummary = latestControlResult.command_kind
    ? `${latestControlResult.command_kind}/${latestControlResult.status || "UNKNOWN"}`
    : "NONE";
  const latestReceiptSummary = latestReceipt.receipt_kind
    ? `${latestReceipt.receipt_kind}@${latestReceipt.timestamp_utc || "NONE"}`
    : "NONE";
  const flowGraph = `BROKER(${brokerSummary.activeRuns.length} active) -> ${options.wpId} [${runtimeStatus} / waiting_on=${waitingOn}]`;
  const syncSummary = [
    `sessions=${sessions.length}`,
    `control=${controlRequests.length}/${controlResults.length}`,
    `receipts=${receipts.length}`,
    `pending=${notificationSummary.totalPending}`,
    `latest_control=${latestControlSummary}`,
    `latest_receipt=${latestReceiptSummary}`,
    `acp=${sessionActivitySummary || "NONE"}`,
    `lane=${relayLaneVerdict ? `${relayLaneVerdict.verdict}/${relayLaneVerdict.reasonCode}` : "NONE"}`,
    `interrupt_budget=${workerInterruptBudget ? `${workerInterruptBudget.currentCycle}/${workerInterruptBudget.maxCycle}` : "NONE"}`,
    `idle=${formatIdleMinutes(latestMechanicalEvent)}m`,
  ].join(" | ");
  const idleThresholdLabel = formatDurationCompact(idleMetrics.idle_threshold_ms);
  const currentWait = idleMetrics.downtime_attribution?.current_wait || { bucket: "NONE", duration_ms: null, reason: "NONE" };
  const idleSummary = [
    `review_rtt(last=${formatDurationCompact(idleMetrics.review_response.latest_ms)}|max=${formatDurationCompact(idleMetrics.review_response.max_ms)}|open=${idleMetrics.review_response.open_count})`,
    `pass_to_coder(last=${formatDurationCompact(idleMetrics.validator_pass_to_next_coder_action.latest_ms)}|max=${formatDurationCompact(idleMetrics.validator_pass_to_next_coder_action.max_ms)}|waiting=${idleMetrics.validator_pass_to_next_coder_action.waiting_count})`,
    `idle(current=${formatDurationCompact(idleMetrics.current_idle_ms)}|max_gap=${formatDurationCompact(idleMetrics.max_idle_gap_ms)}|gaps>=${idleThresholdLabel}:${idleMetrics.idle_gap_count})`,
    `wall_clock(active=${formatDurationCompact(idleMetrics.downtime_attribution.active_build_ms)}|validator=${formatDurationCompact(idleMetrics.downtime_attribution.validator_wait_ms)}|route=${formatDurationCompact(idleMetrics.downtime_attribution.route_wait_ms)}|dependency=${formatDurationCompact(idleMetrics.downtime_attribution.dependency_wait_ms)}|human=${formatDurationCompact(idleMetrics.downtime_attribution.human_wait_ms)}|repair=${formatDurationCompact(idleMetrics.downtime_attribution.repair_overhead_ms)})`,
    `current_wait(${currentWait.bucket}@${formatDurationCompact(currentWait.duration_ms)}|reason=${currentWait.reason || "NONE"})`,
    `queue(level=${idleMetrics.queue_pressure.level}|score=${idleMetrics.queue_pressure.score}|pending=${idleMetrics.queue_pressure.pending_notification_count}|open_reviews=${idleMetrics.queue_pressure.open_review_count}|open_control=${idleMetrics.queue_pressure.unresolved_control_count})`,
    `drift(dup_receipts=${idleMetrics.drift_markers.duplicate_receipt_count}|open_reviews=${idleMetrics.drift_markers.open_review_count}|open_control=${idleMetrics.drift_markers.unresolved_control_count})`,
  ].join(" | ");
  const timestamp = formatWorkflowDossierTimestamp(new Date());
  const role = String(options.role || "ORCHESTRATOR").trim().toUpperCase();
  const tag = String(options.tag || "ACP_SYNC").trim().toUpperCase() || "ACP_SYNC";
  const surface = String(options.surface || "MECHANICAL").trim() || "MECHANICAL";

  const dossierPath = appendWorkflowDossierEntry({
    repoRoot: rootDir,
    wpId: options.wpId,
    filePath: options.file,
    section: "EXECUTION",
    line: `- [${timestamp}] [${role}] [${tag}] [${surface}] \`${flowGraph}\` | ${syncSummary}`,
  });
  if (!dossierPath) {
    fail(`No open workflow dossier found for ${options.wpId}. Run \`just workflow-dossier-init ${options.wpId}\` first or pass --file.`);
  }
  appendWorkflowDossierEntry({
    repoRoot: rootDir,
    wpId: options.wpId,
    filePath: options.file,
    section: "IDLE",
    line: `- [${timestamp}] [${role}] [IDLE_LEDGER] [${surface}] \`${options.wpId}\` | ${idleSummary}`,
  });
  // RGF-196: piggyback repomem injection on every sync so dossier stays current.
  try {
    runInjectRepomem(rootDir, { wpId: options.wpId, file: options.file });
  } catch (err) {
    // Non-fatal: sync should not fail if repomem injection has an issue.
    console.error(`[workflow-dossier sync] repomem injection warning: ${err.message}`);
  }

  console.log(normalizePath(path.relative(rootDir, dossierPath)) || normalizePath(dossierPath));
}

// RGF-196: Extract repomem conversation_log entries into the workflow dossier.
const REPOMEM_CHECKPOINT_TO_SECTION = {
  SESSION_OPEN: "EXECUTION",
  SESSION_CLOSE: "EXECUTION",
  PRE_TASK: "EXECUTION",
  DECISION: "EXECUTION",
  ERROR: "EXECUTION",
  ABANDON: "EXECUTION",
  INSIGHT: "FINDING",
  RESEARCH_CLOSE: "FINDING",
  CONCERN: "CONCERN",
  ESCALATION: "CONCERN",
};

const REPOMEM_CHECKPOINT_TO_TAG = {
  SESSION_OPEN: "REPOMEM_OPEN",
  SESSION_CLOSE: "REPOMEM_CLOSE",
  PRE_TASK: "REPOMEM_PRE",
  DECISION: "REPOMEM_DECISION",
  ERROR: "REPOMEM_ERROR",
  ABANDON: "REPOMEM_ABANDON",
  INSIGHT: "REPOMEM_INSIGHT",
  RESEARCH_CLOSE: "REPOMEM_RESEARCH",
  CONCERN: "REPOMEM_CONCERN",
  ESCALATION: "REPOMEM_ESCALATION",
};

function runInjectRepomem(rootDir, options) {
  const dossierPath = resolveWorkflowDossierPath(rootDir, { wpId: options.wpId, filePath: options.file });
  if (!dossierPath) {
    fail(`No open workflow dossier found for ${options.wpId}. Run \`just workflow-dossier-init ${options.wpId}\` first or pass --file.`);
  }

  const dossierContent = fs.readFileSync(dossierPath, "utf8");

  // Parse OPENED_AT_UTC from dossier metadata.
  const openedMatch = dossierContent.match(/^-\s*OPENED_AT_UTC:\s*(.+)$/m);
  const openedAtUtc = openedMatch ? openedMatch[1].trim() : "";
  if (!openedAtUtc) {
    fail(`Could not parse OPENED_AT_UTC from dossier: ${normalizePath(dossierPath)}`);
  }

  // Open governance memory DB and query conversation_log.
  const { db } = openGovernanceMemoryDb();

  let entries;
  try {
    const sql = `SELECT * FROM conversation_log
      WHERE (wp_id = ? OR wp_id = '')
        AND timestamp_utc >= ?
      ORDER BY timestamp_utc ASC`;
    entries = db.prepare(sql).all(options.wpId, openedAtUtc);
  } finally {
    closeDb(db);
  }

  if (!entries || entries.length === 0) {
    console.log(`[workflow-dossier inject-repomem] No matching repomem entries for ${options.wpId} since ${openedAtUtc}. 0 appended.`);
    return;
  }

  // Read current dossier content for idempotence checks.
  const currentContent = fs.readFileSync(dossierPath, "utf8");
  let appended = 0;

  for (const entry of entries) {
    const section = REPOMEM_CHECKPOINT_TO_SECTION[entry.checkpoint_type];
    const tag = REPOMEM_CHECKPOINT_TO_TAG[entry.checkpoint_type];
    if (!section || !tag) continue;

    const timestamp = formatWorkflowDossierTimestamp(entry.timestamp_utc);
    const role = String(entry.role || "ORCHESTRATOR").trim().toUpperCase();
    const topic = String(entry.topic || "").trim().slice(0, 200);
    const contentPreview = String(entry.content || "").trim().slice(0, 200);
    const display = contentPreview && contentPreview !== topic
      ? `${topic} :: ${contentPreview}`
      : topic;
    const sessionId = String(entry.session_id || "").trim();

    // Idempotence: skip if a line with this session_id and timestamp already exists.
    const dedupeMarker = `[${tag}] [GOVERNANCE_MEMORY] [${sessionId}]`;
    if (currentContent.includes(dedupeMarker) && currentContent.includes(timestamp)) {
      continue;
    }

    const line = `- [${timestamp}] [${role}] [${tag}] [GOVERNANCE_MEMORY] [${sessionId}] ${display}`;
    appendWorkflowDossierEntry({
      repoRoot: rootDir,
      wpId: options.wpId,
      filePath: options.file,
      section,
      line,
    });
    appended++;
  }

  console.log(`[workflow-dossier inject-repomem] ${appended} entries appended to ${normalizePath(path.relative(rootDir, dossierPath))} (${entries.length} matched, ${entries.length - appended} skipped as duplicates).`);
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
  const runtime = runtimePath && fs.existsSync(path.resolve(rootDir, runtimePath))
    ? parseJsonFile(path.resolve(rootDir, runtimePath)) : {};
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
  return { summary, receipts, controlResults };
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

function buildCostTable(metrics) {
  const dt = metrics;
  const totalMin = dt.wall_clock_minutes || 0;
  const activeMin = dt.product_active_minutes || 0;
  const repairMin = dt.repair_minutes || 0;
  const valMin = dt.validator_wait_minutes || 0;
  const routeMin = dt.route_wait_minutes || 0;
  const coderMin = dt.coder_wait_minutes || 0;
  const pollingMin = routeMin + coderMin;
  const fixMin = repairMin;
  const tokIn = dt.token_input_total || 0;
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
  return rows.join("\n");
}

function buildComparisonTable(metrics) {
  const rows = [
    `| Metric | This WP | Notes |`,
    `|---|---|---|`,
    `| Wall clock (min) | ${fmtMin(metrics.wall_clock_minutes)} | |`,
    `| Microtask count | ${metrics.mt_count ?? "N/A"} | |`,
    `| Fix cycles | ${metrics.fix_cycles ?? 0} | |`,
    `| Governed receipts | ${metrics.receipt_count ?? 0} | |`,
    `| ACP commands | ${metrics.acp_commands ?? 0} | failures: ${metrics.acp_failures ?? 0} |`,
    `| Session restarts | ${metrics.session_restarts ?? 0} | |`,
    `| Tokens in | ${fmtTokens(metrics.token_input_total)} | |`,
    `| Tokens out | ${fmtTokens(metrics.token_output_total)} | |`,
    `| Turns | ${metrics.token_turn_count ?? 0} | |`,
  ];
  return rows.join("\n");
}

function runAutofillCosts(rootDir, options) {
  const dossierPath = resolveWorkflowDossierPath(rootDir, { wpId: options.wpId, filePath: options.file });
  if (!dossierPath) {
    fail(`No workflow dossier found for ${options.wpId}. Pass --file for closed dossiers.`);
  }

  const { summary, receipts, controlResults } = loadWpTimelineData(rootDir, options.wpId);
  const metrics = buildWpMetrics({ wpId: options.wpId, summary, receipts, controlResults });

  let content = fs.readFileSync(dossierPath, "utf8");
  let replaced = 0;

  // Replace section 14 — Cost Attribution
  const costSectionRe = /(## 14\. Cost Attribution\n+)\|[^\n]*\n\|[-| ]+\n(?:\|[^\n]*\n)*/;
  if (costSectionRe.test(content)) {
    content = content.replace(costSectionRe, `$1${buildCostTable(metrics)}\n`);
    replaced++;
  }

  // Replace section 15 — Comparison Table
  const compSectionRe = /(## 15\. Comparison Table[^\n]*\n+)\|[^\n]*\n\|[-| ]+\n(?:\|[^\n]*\n)*/;
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
  runSync(rootDir, options);
}

main();
