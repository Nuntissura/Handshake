import fs from 'node:fs';
import path from 'node:path';
import { GOV_ROOT_REPO_REL } from './runtime-paths.mjs';

export const SPEC_DEBT_REGISTRY_PATH = path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'records', 'SPEC_DEBT_REGISTRY.md');

function parsePipeRecord(item) {
  const record = {};
  for (const part of String(item || '').split('|')) {
    const trimmed = part.trim();
    if (!trimmed) continue;
    const idx = trimmed.indexOf(':');
    if (idx === -1) continue;
    const key = trimmed.slice(0, idx).trim().toUpperCase().replace(/\s+/g, '_');
    const value = trimmed.slice(idx + 1).trim();
    record[key] = value;
  }
  return record;
}

function extractDebtItems(content) {
  const lines = String(content || '').split(/\r?\n/);
  const headingRe = /^##\s+DEBT_ROWS\b/i;
  const startIndex = lines.findIndex((line) => headingRe.test(line));
  if (startIndex === -1) return { found: false, items: [] };

  const items = [];
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (/^##\s+\S/.test(line)) break;
    const match = line.match(/^\s*-\s+(.+)\s*$/);
    if (match) items.push((match[1] || '').trim());
  }
  return { found: true, items };
}

function replaceDebtItems(content, items) {
  const lines = String(content || "").split(/\r?\n/);
  const headingRe = /^##\s+DEBT_ROWS\b/i;
  const startIndex = lines.findIndex((line) => headingRe.test(line));
  if (startIndex === -1) {
    throw new Error(`SPEC_DEBT_REGISTRY missing ## DEBT_ROWS heading: ${SPEC_DEBT_REGISTRY_PATH.replace(/\\/g, "/")}`);
  }

  let endIndex = lines.length;
  for (let index = startIndex + 1; index < lines.length; index += 1) {
    if (/^##\s+\S/.test(lines[index])) {
      endIndex = index;
      break;
    }
  }

  const replacement = items.length > 0 ? items.map((item) => `- ${item}`) : ["- NONE"];
  return [
    ...lines.slice(0, startIndex + 1),
    ...replacement,
    ...lines.slice(endIndex),
  ].join("\n");
}

export function loadSpecDebtRegistry(registryPath = SPEC_DEBT_REGISTRY_PATH) {
  const errors = [];
  if (!fs.existsSync(registryPath)) {
    return { errors: [`Missing spec debt registry: ${registryPath.replace(/\\/g, '/')}`], rowsById: new Map(), rows: [] };
  }

  const content = fs.readFileSync(registryPath, 'utf8');
  const debtItems = extractDebtItems(content);
  if (!debtItems.found) {
    errors.push(`SPEC_DEBT_REGISTRY missing ## DEBT_ROWS heading: ${registryPath.replace(/\\/g, '/')}`);
    return { errors, rowsById: new Map(), rows: [] };
  }

  const rows = [];
  const rowsById = new Map();

  for (const item of debtItems.items) {
    if (/^NONE$/i.test(item || '')) continue;
    const record = parsePipeRecord(item);
    const debtId = String(record.DEBT_ID || '').trim();
    const wpId = String(record.WP_ID || '').trim();
    const status = String(record.STATUS || '').trim().toUpperCase();
    const blocking = String(record.BLOCKING || '').trim().toUpperCase();
    const clause = String(record.CLAUSE || '').trim();
    const notes = String(record.NOTES || '').trim();

    if (!/^SPECDEBT-[A-Za-z0-9][A-Za-z0-9_-]*$/i.test(debtId)) {
      errors.push(`SPEC_DEBT_REGISTRY invalid DEBT_ID row: ${item}`);
      continue;
    }
    if (!/^WP-[A-Za-z0-9][A-Za-z0-9-]*$/i.test(wpId)) {
      errors.push(`SPEC_DEBT_REGISTRY ${debtId} has invalid WP_ID: ${wpId || '<missing>'}`);
      continue;
    }
    if (!/^(OPEN|CLOSED)$/.test(status)) {
      errors.push(`SPEC_DEBT_REGISTRY ${debtId} has invalid STATUS: ${status || '<missing>'}`);
      continue;
    }
    if (!/^(YES|NO)$/.test(blocking)) {
      errors.push(`SPEC_DEBT_REGISTRY ${debtId} has invalid BLOCKING: ${blocking || '<missing>'}`);
      continue;
    }
    if (!clause) {
      errors.push(`SPEC_DEBT_REGISTRY ${debtId} missing CLAUSE`);
      continue;
    }
    if (!notes) {
      errors.push(`SPEC_DEBT_REGISTRY ${debtId} missing NOTES`);
      continue;
    }
    if (rowsById.has(debtId)) {
      errors.push(`SPEC_DEBT_REGISTRY duplicate DEBT_ID: ${debtId}`);
      continue;
    }

    const row = { debtId, wpId, status, blocking, clause, notes };
    rows.push(row);
    rowsById.set(debtId, row);
  }

  return { errors, rowsById, rows };
}

export function formatSpecDebtRow({
  debtId,
  wpId,
  status = "OPEN",
  blocking = "NO",
  clause,
  notes,
}) {
  return `DEBT_ID: ${debtId} | WP_ID: ${wpId} | STATUS: ${status} | BLOCKING: ${blocking} | CLAUSE: ${clause} | NOTES: ${notes}`;
}

export function nextSpecDebtId(registry) {
  const numericIds = (registry?.rows || [])
    .map((row) => {
      const match = String(row.debtId || "").match(/^SPECDEBT-(\d+)$/i);
      return match ? Number.parseInt(match[1], 10) : NaN;
    })
    .filter((value) => Number.isFinite(value));
  const nextValue = numericIds.length > 0 ? Math.max(...numericIds) + 1 : 1;
  return `SPECDEBT-${String(nextValue).padStart(4, "0")}`;
}

export function writeSpecDebtRegistryRows(rows, registryPath = SPEC_DEBT_REGISTRY_PATH) {
  if (!fs.existsSync(registryPath)) {
    throw new Error(`Missing spec debt registry: ${registryPath.replace(/\\/g, "/")}`);
  }
  const content = fs.readFileSync(registryPath, "utf8");
  const normalizedRows = rows.map((row) => formatSpecDebtRow(row));
  const nextContent = replaceDebtItems(content, normalizedRows);
  fs.writeFileSync(registryPath, nextContent, "utf8");
}
