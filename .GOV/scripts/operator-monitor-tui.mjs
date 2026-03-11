#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import readline from "node:readline";
import { fileURLToPath } from "node:url";
import { appendWpThreadEntry } from "./wp-thread-append.mjs";
import { normalize, parseJsonFile, parseJsonlFile, validateReceipt, validateRuntimeStatus } from "./wp-communications-lib.mjs";

const TASK_BOARD_PATH = ".GOV/roles_shared/TASK_BOARD.md";
const TRACEABILITY_PATH = ".GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md";
const PACKETS_DIR = ".GOV/task_packets";
const PACKET_STUBS_DIR = ".GOV/task_packets/stubs";

const FILTERS = ["ALL", "ACTIVE", "READY_FOR_DEV", "IN_PROGRESS", "BLOCKED", "STUB", "DONE", "SUPERSEDED"];
const BOARD_ORDER = ["ACTIVE", "READY_FOR_DEV", "IN_PROGRESS", "BLOCKED", "STUB", "DONE", "SUPERSEDED", "OTHER"];
const REFRESH_INTERVAL_MS = 5000;

function readText(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function parsePacketStatus(text) {
  return (
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    ""
  ).trim() || "Ready for Dev";
}

function parseArgs(argv) {
  const options = {
    once: false,
    actorRole: "OPERATOR",
    actorSession: "operator-monitor",
    wpId: "",
    filter: "ALL",
  };
  const args = [...argv];
  while (args.length > 0) {
    const token = args.shift();
    if (token === "--once") {
      options.once = true;
    } else if (token === "--actor-role") {
      options.actorRole = String(args.shift() || "").trim().toUpperCase() || "OPERATOR";
    } else if (token === "--actor-session") {
      options.actorSession = String(args.shift() || "").trim() || "operator-monitor";
    } else if (token === "--wp") {
      options.wpId = String(args.shift() || "").trim();
    } else if (token === "--filter") {
      const value = String(args.shift() || "").trim().toUpperCase();
      if (FILTERS.includes(value)) options.filter = value;
    }
  }
  return options;
}

function parseTaskBoard() {
  const lines = readText(TASK_BOARD_PATH).split(/\r?\n/);
  const entries = [];
  let section = "OTHER";
  for (const line of lines) {
    if (/^##\s+Active\b/.test(line)) {
      section = "ACTIVE";
      continue;
    }
    if (/^##\s+Ready for Dev\b/.test(line)) {
      section = "READY_FOR_DEV";
      continue;
    }
    if (/^##\s+Stub Backlog\b/.test(line)) {
      section = "STUB";
      continue;
    }
    if (/^##\s+In Progress\b/.test(line)) {
      section = "IN_PROGRESS";
      continue;
    }
    if (/^##\s+Done\b/.test(line)) {
      section = "DONE";
      continue;
    }
    if (/^##\s+Blocked\b/.test(line)) {
      section = "BLOCKED";
      continue;
    }
    if (/^##\s+Superseded\b/.test(line)) {
      section = "SUPERSEDED";
      continue;
    }
    const match = line.match(/^\s*-\s+\*\*\[(WP-[^\]]+)\]\*\*\s+-\s+\[([A-Z_]+)\](?:\s+-\s+(.*))?$/);
    if (!match) continue;
    entries.push({
      wpId: match[1],
      boardSection: section,
      boardToken: match[2],
      detail: (match[3] || "").trim(),
    });
  }
  return entries;
}

function parseTraceabilityRegistry() {
  const byBaseWpId = new Map();
  const byWpId = new Map();
  if (!fs.existsSync(TRACEABILITY_PATH)) return { byBaseWpId, byWpId };
  const lines = readText(TRACEABILITY_PATH).split(/\r?\n/);
  for (const line of lines) {
    if (!line.startsWith("|") || /^\|\s*-+/.test(line)) continue;
    const parts = line.split("|").slice(1, -1).map((value) => value.trim());
    if (parts.length < 4) continue;
    const [baseWpId, activePacket] = parts;
    if (!/^WP-/.test(baseWpId) || !activePacket.startsWith(".GOV/")) continue;
    byBaseWpId.set(baseWpId, activePacket);
    byWpId.set(path.basename(activePacket, ".md"), activePacket);
  }
  return { byBaseWpId, byWpId };
}

function resolvePacketPath(wpId, traceability) {
  if (traceability.byWpId.has(wpId)) return traceability.byWpId.get(wpId);
  const official = normalize(path.join(PACKETS_DIR, `${wpId}.md`));
  if (fs.existsSync(official)) return official;
  const stub = normalize(path.join(PACKET_STUBS_DIR, `${wpId}.md`));
  if (fs.existsSync(stub)) return stub;
  if (traceability.byBaseWpId.has(wpId)) return traceability.byBaseWpId.get(wpId);
  return null;
}

function parseThreadEntries(threadPath) {
  if (!threadPath || !fs.existsSync(threadPath)) return [];
  const lines = readText(threadPath).split(/\r?\n/);
  const entries = [];
  let current = null;
  for (const line of lines) {
    const header = line.match(/^\s*-\s+(\d{4}-\d{2}-\d{2}T[^\s]+Z)\s+\|\s+([A-Z_]+)\s+\|\s+session=([^|]+?)(?:\s+\|\s+target=(.+))?\s*$/);
    if (header) {
      if (current) entries.push(current);
      current = {
        timestamp: header[1],
        actorRole: header[2],
        actorSession: header[3].trim(),
        target: (header[4] || "").trim(),
        messageLines: [],
      };
      continue;
    }
    if (current && /^\s{2,}\S/.test(line)) {
      current.messageLines.push(line.trim());
    }
  }
  if (current) entries.push(current);
  return entries;
}

function summarizeSessions(runtime) {
  if (!runtime || !Array.isArray(runtime.active_role_sessions)) return [];
  return runtime.active_role_sessions.map((entry) => ({
    role: entry.role,
    sessionId: entry.session_id,
    authorityKind: entry.authority_kind,
    validatorRoleKind: entry.validator_role_kind,
    state: entry.state,
    worktreeDir: entry.worktree_dir,
    lastHeartbeatAt: entry.last_heartbeat_at,
  }));
}

function parsePacketRecord(packetPath) {
  if (!packetPath || !fs.existsSync(packetPath)) return null;
  const packetText = readText(packetPath);
  const wpId = path.basename(packetPath, ".md");
  const runtimePath = parseSingleField(packetText, "WP_RUNTIME_STATUS_FILE");
  const receiptsPath = parseSingleField(packetText, "WP_RECEIPTS_FILE");
  const threadPath = parseSingleField(packetText, "WP_THREAD_FILE");
  const record = {
    wpId,
    packetPath: normalize(packetPath),
    packetKind: packetPath.includes("/stubs/") ? "STUB" : "OFFICIAL",
    packetStatus: parsePacketStatus(packetText),
    workflowLane: parseSingleField(packetText, "WORKFLOW_LANE") || "UNSPECIFIED",
    executionOwner: parseSingleField(packetText, "EXECUTION_OWNER") || "UNASSIGNED",
    localBranch: parseSingleField(packetText, "LOCAL_BRANCH") || "<pending>",
    localWorktreeDir: parseSingleField(packetText, "LOCAL_WORKTREE_DIR") || "<pending>",
    workflowAuthority: parseSingleField(packetText, "WORKFLOW_AUTHORITY") || "ORCHESTRATOR",
    technicalAdvisor: parseSingleField(packetText, "TECHNICAL_ADVISOR") || "WP_VALIDATOR",
    technicalAuthority: parseSingleField(packetText, "TECHNICAL_AUTHORITY") || "INTEGRATION_VALIDATOR",
    mergeAuthority: parseSingleField(packetText, "MERGE_AUTHORITY") || "INTEGRATION_VALIDATOR",
    wpValidatorOfRecord: parseSingleField(packetText, "WP_VALIDATOR_OF_RECORD") || "<unassigned>",
    integrationValidatorOfRecord: parseSingleField(packetText, "INTEGRATION_VALIDATOR_OF_RECORD") || "<unassigned>",
    communicationDir: parseSingleField(packetText, "WP_COMMUNICATION_DIR"),
    threadPath,
    runtimePath,
    receiptsPath,
  };

  try {
    if (runtimePath && fs.existsSync(runtimePath)) {
      const runtime = parseJsonFile(runtimePath);
      record.runtimeValidationErrors = validateRuntimeStatus(runtime);
      record.runtime = runtime;
    } else {
      record.runtime = null;
      record.runtimeValidationErrors = [];
    }
  } catch (error) {
    record.runtime = null;
    record.runtimeValidationErrors = [error.message];
  }

  try {
    if (receiptsPath && fs.existsSync(receiptsPath)) {
      const receipts = parseJsonlFile(receiptsPath);
      record.receiptValidationErrors = receipts.flatMap((entry, index) =>
        validateReceipt(entry).map((message) => `line ${index + 1}: ${message}`)
      );
      record.receipts = receipts;
    } else {
      record.receipts = [];
      record.receiptValidationErrors = [];
    }
  } catch (error) {
    record.receipts = [];
    record.receiptValidationErrors = [error.message];
  }

  record.threadEntries = parseThreadEntries(threadPath);
  record.lastThreadEntry = record.threadEntries.at(-1) || null;
  record.lastReceipt = record.receipts.at(-1) || null;
  record.lastActivityAt = [
    record.runtime?.last_event_at || null,
    record.lastThreadEntry?.timestamp || null,
    record.lastReceipt?.timestamp_utc || null,
  ]
    .filter(Boolean)
    .sort()
    .at(-1) || null;

  return record;
}

function loadMonitorModel() {
  const traceability = parseTraceabilityRegistry();
  const boardEntries = parseTaskBoard();
  const records = boardEntries.map((entry) => {
    const packetPath = resolvePacketPath(entry.wpId, traceability);
    const packetRecord = parsePacketRecord(packetPath);
    return {
      ...entry,
      packetPath,
      packetRecord,
      sessions: summarizeSessions(packetRecord?.runtime),
      stale: Boolean(packetRecord?.runtime?.stale_after && new Date(packetRecord.runtime.stale_after) < new Date()),
    };
  });
  records.sort((left, right) => {
    const leftIndex = BOARD_ORDER.indexOf(left.boardSection);
    const rightIndex = BOARD_ORDER.indexOf(right.boardSection);
    if (leftIndex !== rightIndex) return leftIndex - rightIndex;
    return left.wpId.localeCompare(right.wpId);
  });
  return records;
}

function filterRecords(records, filter) {
  if (filter === "ALL") return records;
  if (filter === "ACTIVE") return records.filter((record) => record.boardSection === "ACTIVE" || record.sessions.length > 0);
  return records.filter((record) => record.boardSection === filter);
}

function truncate(value, width) {
  const text = String(value ?? "");
  if (width <= 0) return "";
  if (text.length <= width) return text.padEnd(width, " ");
  if (width === 1) return text.slice(0, 1);
  return `${text.slice(0, width - 1)}…`;
}

function wrapText(text, width) {
  const source = String(text || "");
  if (!source) return [""];
  if (width <= 4) return [source.slice(0, width)];
  const words = source.split(/\s+/).filter(Boolean);
  const lines = [];
  let current = "";
  for (const word of words) {
    if (!current) {
      current = word;
      continue;
    }
    if (`${current} ${word}`.length <= width) {
      current += ` ${word}`;
    } else {
      lines.push(current);
      current = word;
    }
  }
  if (current) lines.push(current);
  return lines;
}

function renderList(records, selectedIndex, width, height) {
  const rows = [];
  const maxRows = Math.max(1, height);
  if (records.length === 0) {
    rows.push(truncate("No WPs in this filter.", width));
    while (rows.length < maxRows) rows.push(" ".repeat(width));
    return rows;
  }
  const start = Math.max(0, Math.min(selectedIndex - Math.floor(maxRows / 2), Math.max(0, records.length - maxRows)));
  const visible = records.slice(start, start + maxRows);
  for (let index = 0; index < visible.length; index += 1) {
    const record = visible[index];
    const globalIndex = start + index;
    const marker = globalIndex === selectedIndex ? ">" : " ";
    const stale = record.stale ? "!" : " ";
    const latest = record.packetRecord?.lastActivityAt ? record.packetRecord.lastActivityAt.slice(11, 16) : "-----";
    const threadCount = record.packetRecord?.threadEntries?.length || 0;
    const receiptCount = record.packetRecord?.receipts?.length || 0;
    const sessionCount = record.sessions?.length || 0;
    const line = `${marker}${stale} ${record.wpId} [${record.boardSection}] T${threadCount} R${receiptCount} S${sessionCount} ${latest}`;
    rows.push(globalIndex === selectedIndex ? `\x1b[7m${truncate(line, width)}\x1b[0m` : truncate(line, width));
  }
  while (rows.length < maxRows) rows.push(" ".repeat(width));
  return rows;
}

function buildDetailLines(record, width, detailView) {
  if (!record) return ["No WP selected."];
  const packet = record.packetRecord;
  const runtime = packet?.runtime;
  const lines = [
    `${record.wpId} | board=${record.boardSection} | token=${record.boardToken}`,
    `packet=${packet?.packetKind || "UNKNOWN"} | packet_status=${packet?.packetStatus || "<none>"}`,
    `lane=${packet?.workflowLane || "UNSPECIFIED"} | owner=${packet?.executionOwner || "UNASSIGNED"}`,
    `workflow=${packet?.workflowAuthority || "ORCHESTRATOR"} | tech=${packet?.technicalAuthority || "INTEGRATION_VALIDATOR"} | merge=${packet?.mergeAuthority || "INTEGRATION_VALIDATOR"}`,
    `wpval=${packet?.wpValidatorOfRecord || "<unassigned>"} | ival=${packet?.integrationValidatorOfRecord || "<unassigned>"}`,
    `branch=${packet?.localBranch || "<pending>"}`,
    `worktree=${packet?.localWorktreeDir || "<pending>"}`,
  ];
  if (runtime) {
    lines.push(`runtime=${runtime.runtime_status}/${runtime.current_phase} | next=${runtime.next_expected_actor} | waiting_on=${runtime.waiting_on}`);
    lines.push(`validator_trigger=${runtime.validator_trigger} | ready=${runtime.ready_for_validation ? "YES" : "NO"} | stale=${record.stale ? "YES" : "NO"}`);
  } else {
    lines.push("runtime=<none>");
  }
  lines.push("");

  if (detailView === "THREAD") {
    lines.push("THREAD");
    const recent = (packet?.threadEntries || []).slice(-8);
    if (recent.length === 0) {
      lines.push("No thread entries.");
    } else {
      for (const entry of recent) {
        lines.push(`${entry.timestamp} | ${entry.actorRole} | ${entry.actorSession}${entry.target ? ` | ${entry.target}` : ""}`);
        for (const bodyLine of entry.messageLines) lines.push(`  ${bodyLine}`);
      }
    }
  } else if (detailView === "RECEIPTS") {
    lines.push("RECEIPTS");
    const recent = (packet?.receipts || []).slice(-8);
    if (recent.length === 0) {
      lines.push("No receipts.");
    } else {
      for (const entry of recent) {
        lines.push(`${entry.timestamp_utc} | ${entry.actor_role} | ${entry.receipt_kind}`);
        lines.push(`  ${entry.summary}`);
      }
    }
  } else {
    lines.push("SESSIONS");
    if ((record.sessions || []).length === 0) {
      lines.push("No active sessions recorded.");
    } else {
      for (const session of record.sessions) {
        lines.push(`${session.role} | ${session.sessionId} | ${session.state} | ${session.authorityKind}${session.validatorRoleKind ? `/${session.validatorRoleKind}` : ""}`);
        lines.push(`  ${session.worktreeDir}`);
      }
    }
    if (packet?.runtimeValidationErrors?.length) {
      lines.push("");
      lines.push("RUNTIME VALIDATION ERRORS");
      for (const error of packet.runtimeValidationErrors.slice(0, 5)) lines.push(`- ${error}`);
    }
    if (packet?.receiptValidationErrors?.length) {
      lines.push("");
      lines.push("RECEIPT VALIDATION ERRORS");
      for (const error of packet.receiptValidationErrors.slice(0, 5)) lines.push(`- ${error}`);
    }
  }

  return lines.flatMap((line) => wrapText(line, width));
}

function renderScreen(records, uiState) {
  const filtered = filterRecords(records, uiState.filter);
  if (uiState.selectedIndex >= filtered.length) uiState.selectedIndex = Math.max(0, filtered.length - 1);
  const selected = filtered[uiState.selectedIndex] || null;
  const columns = process.stdout.columns || 120;
  const rows = process.stdout.rows || 35;
  const leftWidth = Math.max(38, Math.floor(columns * 0.42));
  const rightWidth = Math.max(40, columns - leftWidth - 3);
  const bodyHeight = Math.max(12, rows - 6);

  const counts = FILTERS.map((filter) => `${filter}:${filterRecords(records, filter).length}`).join("  ");
  const header = `Operator Monitor | filter=${uiState.filter} | view=${uiState.detailView} | actor=${uiState.actorRole}/${uiState.actorSession}`;
  const help = "j/k move  [/ ] filter  1 sessions  2 thread  3 receipts  m message  r refresh  q quit";

  const leftLines = renderList(filtered, uiState.selectedIndex, leftWidth, bodyHeight);
  const rightLines = buildDetailLines(selected, rightWidth, uiState.detailView).slice(0, bodyHeight);
  while (rightLines.length < bodyHeight) rightLines.push("");

  const frame = [header, counts, help, "-".repeat(columns)];
  for (let index = 0; index < bodyHeight; index += 1) {
    frame.push(`${leftLines[index]} | ${truncate(rightLines[index], rightWidth)}`);
  }
  frame.push("-".repeat(columns));
  frame.push(selected ? `Selected: ${selected.wpId} | packet=${selected.packetPath || "<none>"} | last_activity=${selected.packetRecord?.lastActivityAt || "n/a"}` : "Selected: none");
  return frame.join(os.EOL);
}

function renderOnce(records, options) {
  const uiState = {
    filter: options.filter,
    selectedIndex: 0,
    detailView: "SESSIONS",
    actorRole: options.actorRole,
    actorSession: options.actorSession,
  };
  const filtered = filterRecords(records, uiState.filter);
  if (options.wpId) {
    const index = filtered.findIndex((record) => record.wpId === options.wpId);
    if (index >= 0) uiState.selectedIndex = index;
  }
  console.log(renderScreen(records, uiState));
}

async function promptForMessage(selectedRecord, uiState) {
  if (!selectedRecord || !selectedRecord.packetRecord?.threadPath) return null;
  if (process.stdin.isTTY) process.stdin.setRawMode(false);
  process.stdout.write("\x1b[?25h");
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  const answer = await new Promise((resolve) => rl.question(`message ${selectedRecord.wpId}> `, resolve));
  rl.close();
  if (process.stdin.isTTY) process.stdin.setRawMode(true);
  process.stdout.write("\x1b[?25l");
  const trimmed = String(answer || "").trim();
  if (!trimmed) return null;
  const targetMatch = trimmed.match(/^(@\S+)\s+([\s\S]+)$/);
  return appendWpThreadEntry({
    wpId: selectedRecord.wpId,
    actorRole: uiState.actorRole,
    actorSession: uiState.actorSession,
    message: targetMatch ? targetMatch[2] : trimmed,
    target: targetMatch ? targetMatch[1] : "",
  });
}

async function runInteractive(options) {
  if (!process.stdout.isTTY || !process.stdin.isTTY) {
    renderOnce(loadMonitorModel(), options);
    return;
  }

  const uiState = {
    filter: options.filter,
    selectedIndex: 0,
    detailView: "SESSIONS",
    actorRole: options.actorRole,
    actorSession: options.actorSession,
  };

  let records = loadMonitorModel();
  const refresh = () => {
    records = loadMonitorModel();
    process.stdout.write("\x1b[2J\x1b[H");
    process.stdout.write(renderScreen(records, uiState));
  };

  process.stdout.write("\x1b[?1049h\x1b[?25l");
  process.stdin.setRawMode(true);
  process.stdin.resume();
  process.stdin.setEncoding("utf8");
  refresh();

  const timer = setInterval(refresh, REFRESH_INTERVAL_MS);
  const cleanup = () => {
    clearInterval(timer);
    if (process.stdin.isTTY) process.stdin.setRawMode(false);
    process.stdout.write("\x1b[?25h\x1b[?1049l");
  };

  process.on("SIGINT", () => {
    cleanup();
    process.exit(0);
  });

  process.stdin.on("data", async (chunk) => {
    const key = String(chunk || "");
    const filtered = filterRecords(records, uiState.filter);
    if (key === "q") {
      cleanup();
      process.exit(0);
    } else if (key === "j" || key === "\u001b[B") {
      uiState.selectedIndex = Math.min(filtered.length - 1, uiState.selectedIndex + 1);
      refresh();
    } else if (key === "k" || key === "\u001b[A") {
      uiState.selectedIndex = Math.max(0, uiState.selectedIndex - 1);
      refresh();
    } else if (key === "]") {
      const index = FILTERS.indexOf(uiState.filter);
      uiState.filter = FILTERS[(index + 1) % FILTERS.length];
      uiState.selectedIndex = 0;
      refresh();
    } else if (key === "[") {
      const index = FILTERS.indexOf(uiState.filter);
      uiState.filter = FILTERS[(index - 1 + FILTERS.length) % FILTERS.length];
      uiState.selectedIndex = 0;
      refresh();
    } else if (key === "1") {
      uiState.detailView = "SESSIONS";
      refresh();
    } else if (key === "2") {
      uiState.detailView = "THREAD";
      refresh();
    } else if (key === "3") {
      uiState.detailView = "RECEIPTS";
      refresh();
    } else if (key === "r") {
      refresh();
    } else if (key === "m") {
      const selected = filtered[uiState.selectedIndex] || null;
      await promptForMessage(selected, uiState);
      records = loadMonitorModel();
      refresh();
    }
  });
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const records = loadMonitorModel();
  if (options.once) {
    renderOnce(records, options);
    return;
  }
  await runInteractive(options);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(`[OPERATOR_MONITOR] ${error.message}`);
    process.exit(1);
  });
}
