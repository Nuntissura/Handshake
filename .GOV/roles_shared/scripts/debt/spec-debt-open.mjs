#!/usr/bin/env node
import path from "node:path";
import {
  formatSpecDebtRow,
  loadSpecDebtRegistry,
  nextSpecDebtId,
  writeSpecDebtRegistryRows,
} from "../lib/spec-debt-registry-lib.mjs";
import {
  formatUpdatedPacket,
  parseClauseRows,
  readPacket,
  writePacket,
} from "../lib/spec-debt-packet-lib.mjs";

const [wpId, clauseSelectorRaw, notesRaw, blockingRaw = "NO"] = process.argv.slice(2);

function fail(message) {
  console.error(`[SPEC_DEBT_OPEN] ${message}`);
  process.exit(1);
}

function sanitizeInline(value) {
  return String(value || "").replace(/\r?\n/g, " ").replace(/\|/g, "/").trim();
}

function findClauseRow(rows, clauseSelector) {
  const exactMatches = rows.map((row, index) => ({ row, index })).filter(({ row }) => row.clause === clauseSelector);
  if (exactMatches.length === 1) return exactMatches[0];
  const caseInsensitive = rows.map((row, index) => ({ row, index })).filter(({ row }) => row.clause.toLowerCase() === clauseSelector.toLowerCase());
  if (caseInsensitive.length === 1) return caseInsensitive[0];
  const containsMatches = rows.map((row, index) => ({ row, index })).filter(({ row }) => row.clause.toLowerCase().includes(clauseSelector.toLowerCase()));
  if (containsMatches.length === 1) return containsMatches[0];
  return null;
}

if (!/^WP-[A-Za-z0-9][A-Za-z0-9-]*$/i.test(wpId || "")) {
  fail("Usage: node .GOV/roles_shared/scripts/debt/spec-debt-open.mjs WP-{ID} \"<clause>\" \"<notes>\" <YES|NO>");
}

const clauseSelector = sanitizeInline(clauseSelectorRaw);
const notes = sanitizeInline(notesRaw);
const blocking = String(blockingRaw || "").trim().toUpperCase();
if (!clauseSelector) fail("Clause selector must not be empty");
if (!notes) fail("Notes must not be empty");
if (!/^(YES|NO)$/.test(blocking)) fail("Blocking flag must be YES or NO");

const packetPath = path.join(".GOV", "task_packets", `${wpId}.md`);
const packetText = readPacket(packetPath);
const clauseRows = parseClauseRows(packetText);
if (clauseRows.length === 0) {
  fail(`Packet has no CLAUSE_CLOSURE_MATRIX rows: ${packetPath.replace(/\\/g, "/")}`);
}

const clauseMatch = findClauseRow(clauseRows, clauseSelector);
if (!clauseMatch) {
  fail(`Could not resolve a unique clause row for selector: ${clauseSelector}`);
}

const registry = loadSpecDebtRegistry();
if (registry.errors.length > 0) {
  fail(registry.errors.join("; "));
}

const debtId = nextSpecDebtId(registry);
const updatedRows = clauseRows.map((row) => {
  if (row !== clauseMatch.row) return row;
  return {
    ...row,
    debtIds: [...row.debtIds, debtId],
  };
});

const nextRegistryRows = [
  ...registry.rows,
  {
    debtId,
    wpId,
    status: "OPEN",
    blocking,
    clause: clauseMatch.row.clause,
    notes,
  },
];

const allDebtIds = [...new Set(updatedRows.flatMap((row) => row.debtIds))];
const blockingSpecDebt = allDebtIds.some((id) => {
  const row = nextRegistryRows.find((entry) => entry.debtId === id);
  return row?.blocking === "YES";
}) ? "YES" : "NO";
let nextPacketText;
try {
  nextPacketText = formatUpdatedPacket(packetText, updatedRows, {
    openSpecDebt: allDebtIds.length > 0 ? "YES" : "NO",
    blockingSpecDebt,
    debtIds: allDebtIds,
  });
} catch (error) {
  fail(error.message || String(error));
}
writePacket(packetPath, nextPacketText);

writeSpecDebtRegistryRows(nextRegistryRows);

console.log(`spec-debt-open ok: ${formatSpecDebtRow({ debtId, wpId, status: "OPEN", blocking, clause: clauseMatch.row.clause, notes })}`);
