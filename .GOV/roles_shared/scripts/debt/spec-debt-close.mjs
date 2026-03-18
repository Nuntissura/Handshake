#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { GOV_ROOT_REPO_REL } from "../lib/runtime-paths.mjs";
import { loadSpecDebtRegistry, writeSpecDebtRegistryRows } from "../lib/spec-debt-registry-lib.mjs";
import { parseClauseRows, readPacket } from "../lib/spec-debt-packet-lib.mjs";

const debtId = String(process.argv[2] || "").trim();

function fail(message) {
  console.error(`[SPEC_DEBT_CLOSE] ${message}`);
  process.exit(1);
}

if (!/^SPECDEBT-[A-Za-z0-9][A-Za-z0-9_-]*$/i.test(debtId)) {
  fail("Usage: node .GOV/roles_shared/scripts/debt/spec-debt-close.mjs SPECDEBT-0001");
}

const registry = loadSpecDebtRegistry();
if (registry.errors.length > 0) fail(registry.errors.join("; "));

const target = registry.rowsById.get(debtId);
if (!target) fail(`Unknown debt id: ${debtId}`);
if (target.status === "CLOSED") {
  console.log(`spec-debt-close ok: ${debtId} already CLOSED`);
  process.exit(0);
}

const packetsDir = path.join(GOV_ROOT_REPO_REL, "task_packets");
const referencingPackets = [];
if (fs.existsSync(packetsDir)) {
  const packetFiles = fs.readdirSync(packetsDir).filter((name) => name.endsWith(".md") && name !== "README.md");
  for (const name of packetFiles) {
    const packetPath = path.join(packetsDir, name);
    const packetText = readPacket(packetPath);
    const rows = parseClauseRows(packetText);
    if (rows.some((row) => row.debtIds.includes(debtId)) || new RegExp(`^\\s*-\\s*(?:\\*\\*)?DEBT_IDS(?:\\*\\*)?\\s*:\\s*.*\\b${debtId}\\b`, "mi").test(packetText)) {
      referencingPackets.push(packetPath.replace(/\\/g, "/"));
    }
  }
}

if (referencingPackets.length > 0) {
  fail(`Cannot close ${debtId}; it is still referenced by packet(s): ${referencingPackets.join(", ")}`);
}

writeSpecDebtRegistryRows(registry.rows.map((row) => (
  row.debtId === debtId
    ? { ...row, status: "CLOSED" }
    : row
)));

console.log(`spec-debt-close ok: ${debtId} CLOSED`);
