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
  buildWpTimelineEntries,
  buildWpTimelineSpans,
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
} from "./workflow-dossier-lib.mjs";
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
  fail("Usage: node .GOV/roles_shared/scripts/audit/workflow-dossier.mjs <init|note|sync> WP-{ID} [args]");
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
    if ((command === "note" || command === "sync") && token === "--file") {
      options.file = String(args.shift() || "").trim();
      continue;
    }
    if (command === "note" && token === "--time") {
      options.time = String(args.shift() || "").trim();
      continue;
    }
    fail(`Unknown argument: ${token}`);
  }

  if (!["init", "note", "sync"].includes(command)) {
    usage();
  }
  if (command === "init" && options.output && options.autoOutput) {
    fail("Use either --output or --auto-output, not both.");
  }

  return options;
}

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
      line: `- [${timestamp}] [${role}] [${tag || "CONCERN"}] ${summary}`,
    };
  }
  return {
    section,
    line: `- [${timestamp}] [${role}] [${tag || "GENERAL"}] ${summary}`,
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
  console.log(normalizePath(path.relative(rootDir, dossierPath)) || normalizePath(dossierPath));
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
  runSync(rootDir, options);
}

main();
