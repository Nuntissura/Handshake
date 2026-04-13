#!/usr/bin/env node
import path from "node:path";
import { resolveWorkPacketPath, WORK_PACKET_STORAGE_ROOT_REPO_REL } from "../lib/runtime-paths.mjs";
import { loadSpecDebtRegistry } from "../lib/spec-debt-registry-lib.mjs";
import {
  formatUpdatedPacket,
  parseClauseRows,
  readPacket,
  writePacket,
} from "../lib/spec-debt-packet-lib.mjs";

import { registerFailCaptureHook, failWithMemory } from "../lib/fail-capture-lib.mjs";

registerFailCaptureHook("spec-debt-sync.mjs", { role: "SHARED" });

const wpId = String(process.argv[2] || "").trim();

function fail(message) {
  failWithMemory("spec-debt-sync.mjs", message, { role: "SHARED" });
}

if (!/^WP-[A-Za-z0-9][A-Za-z0-9-]*$/i.test(wpId)) {
  fail("Usage: node .GOV/roles_shared/scripts/debt/spec-debt-sync.mjs WP-{ID}");
}

const resolved = resolveWorkPacketPath(wpId);
const packetPath = resolved?.packetPath || path.join(WORK_PACKET_STORAGE_ROOT_REPO_REL, `${wpId}.md`);
const packetText = readPacket(packetPath);
const clauseRows = parseClauseRows(packetText);
const registry = loadSpecDebtRegistry();
if (registry.errors.length > 0) fail(registry.errors.join("; "));

const debtIds = [...new Set(clauseRows.flatMap((row) => row.debtIds))];
for (const debtId of debtIds) {
  const row = registry.rowsById.get(debtId);
  if (!row) fail(`Packet references missing spec debt id: ${debtId}`);
  if (row.wpId !== wpId) fail(`Packet references debt ${debtId} for ${row.wpId}, expected ${wpId}`);
  if (row.status !== "OPEN") fail(`Packet references closed debt ${debtId}; remove it from CLAUSE_CLOSURE_MATRIX before syncing`);
}

const blockingSpecDebt = debtIds.some((debtId) => registry.rowsById.get(debtId)?.blocking === "YES") ? "YES" : "NO";
let nextPacketText;
try {
  nextPacketText = formatUpdatedPacket(packetText, clauseRows, {
    openSpecDebt: debtIds.length > 0 ? "YES" : "NO",
    blockingSpecDebt,
    debtIds,
  });
} catch (error) {
  fail(error.message || String(error));
}
writePacket(packetPath, nextPacketText);

console.log(`spec-debt-sync ok: ${wpId} -> ${debtIds.length > 0 ? debtIds.join(", ") : "NONE"}`);
