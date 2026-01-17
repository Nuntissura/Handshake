import fs from "node:fs";

const TRACE_REGISTRY_PATH = "docs/WP_TRACEABILITY_REGISTRY.md";
const TASK_BOARD_PATH = "docs/TASK_BOARD.md";
const TASK_PACKETS_DIR = "docs/task_packets";
const TASK_PACKET_STUBS_DIR = "docs/task_packets/stubs";

function fail(message, details = []) {
  console.error(`[WP_TRACEABILITY_CHECK] ${message}`);
  for (const line of details) console.error(`  - ${line}`);
  process.exit(1);
}

function parseRegistryTable(content) {
  const map = new Map();
  const lines = content.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (!line.trim().startsWith("|")) continue;
    const parts = line.split("|").slice(1, -1).map((p) => p.trim());
    if (parts.length < 2) continue;
    const baseWpId = parts[0];
    const activePacket = parts[1];
    if (!baseWpId.startsWith("WP-")) continue;
    map.set(baseWpId, { activePacket, lineNumber: index + 1 });
  }
  return map;
}

function listRevisionPacketIds() {
  if (!fs.existsSync(TASK_PACKETS_DIR)) return [];
  const files = fs.readdirSync(TASK_PACKETS_DIR).filter((name) => name.endsWith(".md"));
  return files
    .map((name) => name.slice(0, -3))
    .filter((wpId) => /-v\d+$/.test(wpId));
}

function baseWpIdFromPacketId(wpId) {
  return wpId.replace(/-v\d+$/, "");
}

function packetIdFromPath(packetPath) {
  const parts = packetPath.split("/");
  const last = parts[parts.length - 1] || "";
  return last.endsWith(".md") ? last.slice(0, -3) : last;
}

function isStubPacketPath(packetPath) {
  return packetPath.startsWith(`${TASK_PACKET_STUBS_DIR}/`);
}

function isOfficialPacketPath(packetPath) {
  return packetPath.startsWith(`${TASK_PACKETS_DIR}/`) && !isStubPacketPath(packetPath);
}

function exists(path) {
  try {
    return fs.existsSync(path);
  } catch {
    return false;
  }
}

const registryContent = fs.existsSync(TRACE_REGISTRY_PATH)
  ? fs.readFileSync(TRACE_REGISTRY_PATH, "utf8")
  : "";
const registry = parseRegistryTable(registryContent);

const taskBoardContent = fs.existsSync(TASK_BOARD_PATH) ? fs.readFileSync(TASK_BOARD_PATH, "utf8") : "";

const revisionPacketIds = listRevisionPacketIds();
const baseToRevisionPackets = new Map();
for (const wpId of revisionPacketIds) {
  const baseWpId = baseWpIdFromPacketId(wpId);
  const list = baseToRevisionPackets.get(baseWpId) || [];
  list.push(wpId);
  baseToRevisionPackets.set(baseWpId, list);
}

const violations = [];

for (const [baseWpId, wpIds] of baseToRevisionPackets.entries()) {
  const registryRow = registry.get(baseWpId);
  if (!registryRow) {
    const examples = wpIds.map((id) => `docs/task_packets/${id}.md`).slice(0, 3);
    violations.push(
      `${TRACE_REGISTRY_PATH}: missing Base→Active mapping for ${baseWpId} (examples: ${examples.join(", ")})`
    );
    continue;
  }

  const activePacketPath = registryRow.activePacket;
  const activePacketId = packetIdFromPath(activePacketPath);

  if (isStubPacketPath(activePacketPath)) {
    const expectedOfficial = `${TASK_PACKETS_DIR}/${activePacketId}.md`;
    if (exists(expectedOfficial)) {
      violations.push(
        `${TRACE_REGISTRY_PATH}:${registryRow.lineNumber}: ${baseWpId} still points to stub (${activePacketPath}) but official packet exists (${expectedOfficial})`
      );
    }
    continue;
  }

  if (!isOfficialPacketPath(activePacketPath)) continue;

  if (!exists(activePacketPath)) {
    violations.push(
      `${TRACE_REGISTRY_PATH}:${registryRow.lineNumber}: ${baseWpId} Active Packet file missing (${activePacketPath})`
    );
    continue;
  }

  const stubLine = `- **[${activePacketId}]** - [STUB]`;
  if (taskBoardContent.includes(stubLine)) {
    violations.push(`${TASK_BOARD_PATH}: ${activePacketId} is STUB but is the Active Packet for ${baseWpId}`);
  }
}

if (violations.length > 0) {
  fail("Activation traceability drift detected", violations);
}

console.log("wp-activation-traceability-check ok");
