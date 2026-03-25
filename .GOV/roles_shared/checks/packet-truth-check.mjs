import fs from "node:fs";
import { inferWpIdFromPacketPath } from "../scripts/lib/runtime-paths.mjs";
import { parseMergeProgressionTruth } from "../scripts/lib/merge-progression-truth-lib.mjs";
import { packetRequiresMergeContainmentTruth } from "../scripts/session/session-policy.mjs";

const TRACE_REGISTRY_PATH = ".GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md";
const TASK_BOARD_PATH = ".GOV/roles_shared/records/TASK_BOARD.md";
const TASK_PACKETS_DIR = ".GOV/task_packets";
const TASK_PACKET_STUBS_DIR = ".GOV/task_packets/stubs";

function fail(message, details = []) {
  console.error(`[PACKET_TRUTH_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function normalizePath(value) {
  return String(value || "").replace(/\\/g, "/").trim();
}

function packetIdFromPath(packetPath) {
  return inferWpIdFromPacketPath(normalizePath(packetPath));
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function parseStatus(text) {
  return (
    (text.match(/^\s*-\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*\*\*Status:\*\*\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*-\s*STUB_STATUS\s*:\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*-\s*STATUS\s*:\s*(.+)\s*$/mi) || [])[1] ||
    (text.match(/^\s*STATUS\s*:\s*(.+)\s*$/mi) || [])[1] ||
    ""
  ).trim();
}

function baseWpIdFromPacket(packetId, packetText) {
  const raw = parseSingleField(packetText, "BASE_WP_ID")
    .replace(/\s*\(.*/, "")
    .trim();
  if (raw) return raw;
  return packetId.replace(/-v\d+$/, "").replace(/-\d{8}$/, "");
}

function extractSection(text, heading) {
  const lines = String(text || "").split(/\r?\n/);
  const headingRe = new RegExp(`^##\\s+${heading}\\s*$`, "i");
  const startIndex = lines.findIndex((line) => headingRe.test(line.trim()));
  if (startIndex === -1) return "";

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index].trim())) {
      endIndex = index;
      break;
    }
  }
  return lines.slice(startIndex + 1, endIndex).join("\n");
}

function parseRegistryRows(content) {
  const scopedContent = extractSection(content, "Registry \\(Phase 1\\)");
  const rows = new Map();
  const lines = scopedContent.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (!line.trim().startsWith("|")) continue;
    const parts = line.split("|").slice(1, -1).map((p) => p.trim());
    if (parts.length < 2) continue;
    const baseWpId = parts[0];
    const activePacketPath = normalizePath(parts[1]);
    if (!baseWpId.startsWith("WP-")) continue;
    rows.set(baseWpId, {
      baseWpId,
      activePacketPath,
      activePacketId: packetIdFromPath(activePacketPath),
      lineNumber: index + 1,
    });
  }
  return rows;
}

function parseTaskBoardTokens(content) {
  const tokens = new Map();
  const lines = content.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index].trim();
    const match = line.match(/^- \*\*\[(WP-[^\]]+)\]\*\* - \[([^\]]+)\](?:\s*-\s*.+)?$/);
    if (!match) continue;
    tokens.set(match[1], { token: match[2], lineNumber: index + 1 });
  }
  return tokens;
}

function normalizedStatusKind(status) {
  const value = String(status || "").trim().toUpperCase();
  if (!value) return "UNKNOWN";
  if (value.includes("STUB")) return "STUB";
  if (value.includes("READY FOR DEV")) return "READY_FOR_DEV";
  if (value.includes("IN PROGRESS")) return "IN_PROGRESS";
  if (value.includes("BLOCKED")) return "BLOCKED";
  if (/VALIDATED\s*\(\s*PASS\s*\)/.test(value)) return "VALIDATED_PASS";
  if (/VALIDATED\s*\(\s*FAIL\s*\)/.test(value)) return "VALIDATED_FAIL";
  if (/VALIDATED\s*\(\s*OUTDATED_ONLY\s*\)/.test(value)) return "VALIDATED_OUTDATED_ONLY";
  if (value.includes("DONE") || value.includes("COMPLETE") || value.includes("VALIDATED")) return "DONE";
  return "UNKNOWN";
}

function expectedBoardTokens(statusKind, isStub) {
  if (isStub) return new Set(["STUB"]);
  if (statusKind === "READY_FOR_DEV") return new Set(["READY_FOR_DEV"]);
  if (statusKind === "IN_PROGRESS") return new Set(["IN_PROGRESS"]);
  if (statusKind === "BLOCKED") return new Set(["BLOCKED"]);
  if (statusKind === "VALIDATED_PASS") return new Set(["VALIDATED"]);
  if (statusKind === "VALIDATED_FAIL") return new Set(["FAIL"]);
  if (statusKind === "VALIDATED_OUTDATED_ONLY") return new Set(["OUTDATED_ONLY"]);
  if (statusKind === "DONE") return new Set(["VALIDATED", "FAIL", "OUTDATED_ONLY"]);
  return new Set();
}

function expectedBoardTokensForPacket(packet) {
  if (packet.kind === "stub") return new Set(["STUB"]);

  const mergeTruth = parseMergeProgressionTruth(packet.packetText);
  if (
    packetRequiresMergeContainmentTruth(mergeTruth.packetFormatVersion)
    && normalizedStatusKind(packet.status) === "DONE"
  ) {
    return new Set(["MERGE_PENDING"]);
  }

  return expectedBoardTokens(normalizedStatusKind(packet.status), false);
}

function readPacketInventory(dir, kind) {
  const entries = [];
  if (!fs.existsSync(dir)) return entries;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    let filePath = "";
    let packetId = "";
    if (entry.isDirectory()) {
      if (kind !== "official" || !entry.name.startsWith("WP-")) continue;
      filePath = `${dir}/${entry.name}/packet.md`.replace(/\\/g, "/");
      if (!fs.existsSync(filePath)) continue;
      packetId = entry.name;
    } else if (entry.isFile() && entry.name.endsWith(".md") && entry.name !== "README.md") {
      filePath = `${dir}/${entry.name}`.replace(/\\/g, "/");
      packetId = entry.name.slice(0, -3);
    } else {
      continue;
    }
    const text = fs.readFileSync(filePath, "utf8");
    entries.push({
      kind,
      filePath,
      packetId,
      packetText: text,
      baseWpId: baseWpIdFromPacket(packetId, text),
      status: parseStatus(text),
    });
  }
  return entries;
}

const registryContent = fs.existsSync(TRACE_REGISTRY_PATH)
  ? fs.readFileSync(TRACE_REGISTRY_PATH, "utf8")
  : "";
const taskBoardContent = fs.existsSync(TASK_BOARD_PATH)
  ? fs.readFileSync(TASK_BOARD_PATH, "utf8")
  : "";

const registryRows = parseRegistryRows(registryContent);
const taskBoardTokens = parseTaskBoardTokens(taskBoardContent);

const officialPackets = readPacketInventory(TASK_PACKETS_DIR, "official");
const stubPackets = readPacketInventory(TASK_PACKET_STUBS_DIR, "stub");
const allPackets = [...officialPackets, ...stubPackets];

const packetsByBaseWpId = new Map();
for (const packet of allPackets) {
  const list = packetsByBaseWpId.get(packet.baseWpId) || [];
  list.push(packet);
  packetsByBaseWpId.set(packet.baseWpId, list);
}

const violations = [];

for (const [baseWpId, registryRow] of registryRows.entries()) {
  const activePacket = allPackets.find((packet) => packet.packetId === registryRow.activePacketId);
  if (!activePacket) {
    violations.push(
      `${TRACE_REGISTRY_PATH}:${registryRow.lineNumber}: Active packet missing on disk for ${baseWpId} (${registryRow.activePacketPath})`
    );
    continue;
  }

  if (activePacket.baseWpId !== baseWpId) {
    violations.push(
      `${TRACE_REGISTRY_PATH}:${registryRow.lineNumber}: Active packet ${activePacket.packetId} declares BASE_WP_ID=${activePacket.baseWpId}, expected ${baseWpId}`
    );
  }

  const boardEntry = taskBoardTokens.get(activePacket.packetId);
  if (!boardEntry) {
    violations.push(
      `${TASK_BOARD_PATH}: missing projection entry for active packet ${activePacket.packetId} (BASE_WP_ID=${baseWpId})`
    );
  } else {
    const expected = expectedBoardTokensForPacket(activePacket);
    if (expected.size === 0) {
      violations.push(
        `${activePacket.filePath}: cannot derive Task Board projection from packet status "${activePacket.status}" for active packet ${activePacket.packetId}`
      );
    } else if (!expected.has(boardEntry.token)) {
      violations.push(
        `${TASK_BOARD_PATH}:${boardEntry.lineNumber}: active packet ${activePacket.packetId} has status "${activePacket.status}" but Task Board token is [${boardEntry.token}]`
      );
    }
    if (boardEntry.token === "SUPERSEDED") {
      violations.push(
        `${TASK_BOARD_PATH}:${boardEntry.lineNumber}: active packet ${activePacket.packetId} cannot be marked [SUPERSEDED]`
      );
    }
  }

  const siblings = packetsByBaseWpId.get(baseWpId) || [];
  for (const sibling of siblings) {
    if (sibling.packetId === activePacket.packetId) continue;
    const olderBoardEntry = taskBoardTokens.get(sibling.packetId);
    if (!olderBoardEntry) {
      violations.push(
        `${TASK_BOARD_PATH}: older packet ${sibling.packetId} shares BASE_WP_ID=${baseWpId} with active packet ${activePacket.packetId} but is not marked [SUPERSEDED]`
      );
      continue;
    }
    if (olderBoardEntry.token !== "SUPERSEDED") {
      violations.push(
        `${TASK_BOARD_PATH}:${olderBoardEntry.lineNumber}: older packet ${sibling.packetId} shares BASE_WP_ID=${baseWpId} with active packet ${activePacket.packetId} and must be [SUPERSEDED], not [${olderBoardEntry.token}]`
      );
    }
  }
}

if (violations.length > 0) {
  fail("Packet truth drift detected", violations);
}

console.log("packet-truth-check ok");
