#!/usr/bin/env node
/**
 * Product Governance Snapshot generator (HARD)
 * Spec anchor: Handshake_Master_Spec_v02.125.md 7.5.4.10 + 7.5.4.3
 *
 * Determinism requirements:
 * - Whitelist-only reads (no repo scan; no extras)
 * - No timestamps / wall-clock calls
 * - Stable sorting for all collections
 * - Output bytes: JSON.stringify(obj, null, 2) + "\n"
 */

import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

export const SCHEMA_VERSION = 'hsk.product_governance_snapshot@0.1';
export const DEFAULT_OUTPUT_PATH = '.GOV/roles_shared/PRODUCT_GOVERNANCE_SNAPSHOT.json';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCRIPT_DIR = path.dirname(SCRIPT_PATH);
const REPO_ROOT = path.resolve(SCRIPT_DIR, '..', '..');

const compareStrings = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

const toPosixRelPath = (p) => p.replace(/\\/g, '/').replace(/^\.\//, '');

const normalizeRelPath = (relPath) => {
  const raw = (relPath ?? '').trim();
  if (!raw) throw new Error('INVALID_PATH: empty');
  if (path.isAbsolute(raw)) throw new Error(`INVALID_PATH: absolute paths are forbidden (${raw})`);

  const normalized = toPosixRelPath(path.normalize(raw));
  if (normalized.startsWith('..')) {
    throw new Error(`INVALID_PATH: path escapes repo root (${relPath})`);
  }
  return normalized;
};

const absFromRel = (relPath) => path.normalize(path.resolve(REPO_ROOT, relPath));

const shaHex = (algorithm, buf) => crypto.createHash(algorithm).update(buf).digest('hex');

export const resolveSpecFileRelPathFromSpecCurrent = (specCurrentText) => {
  const text = specCurrentText ?? '';
  const bold = text.match(/\*\*(Handshake_Master_Spec_v[0-9]+\.[0-9]+\.md)\*\*/);
  if (bold?.[1]) return normalizeRelPath(bold[1]);
  const plain = text.match(/\bHandshake_Master_Spec_v[0-9]+\.[0-9]+\.md\b/);
  if (plain?.[0]) return normalizeRelPath(plain[0]);
  throw new Error('SPEC_CURRENT_UNPARSEABLE: no Handshake_Master_Spec_vNN.NNN.md found');
};

const listValidatorGateJsonRelPaths = () => {
  const dirRel = '.GOV/validator_gates';
  const dirAbs = absFromRel(dirRel);
  if (!fs.existsSync(dirAbs)) return [];
  const entries = fs.readdirSync(dirAbs, { withFileTypes: true });
  const files = entries
    .filter((e) => e.isFile() && e.name.toLowerCase().endsWith('.json'))
    .map((e) => e.name)
    .sort(compareStrings);
  return files.map((name) => normalizeRelPath(`${dirRel}/${name}`));
};

export const computeWhitelistedInputRelPaths = () => {
  const specCurrentRel = '.GOV/roles_shared/SPEC_CURRENT.md';
  const specCurrentAbs = absFromRel(specCurrentRel);
  if (!fs.existsSync(specCurrentAbs)) {
    throw new Error(`INPUT_MISSING: ${specCurrentRel}`);
  }

  const specCurrentText = fs.readFileSync(specCurrentAbs, 'utf8');
  const specFileRel = resolveSpecFileRelPathFromSpecCurrent(specCurrentText);

  const fixed = [
    specCurrentRel,
    specFileRel,
    '.GOV/roles_shared/TASK_BOARD.md',
    '.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md',
    '.GOV/roles_shared/SIGNATURE_AUDIT.md',
    '.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json',
  ].map(normalizeRelPath);

  const validatorGateJsons = listValidatorGateJsonRelPaths();
  const all = [...fixed, ...validatorGateJsons].map(normalizeRelPath);

  // Ensure inputs exist (validator gate JSONs are filtered by readdir, so they exist).
  all.forEach((rel) => {
    const abs = absFromRel(rel);
    if (!fs.existsSync(abs)) throw new Error(`INPUT_MISSING: ${rel}`);
  });

  return { inputsRelPaths: all, specFileRelPath: specFileRel };
};

const parseTaskBoardEntries = (taskBoardText) => {
  const entries = [];
  const lines = (taskBoardText ?? '').split('\n');
  const re = /^\s*-\s+\*\*\[([^\]]+)\]\*\*\s+-\s+\[([^\]]+)\]/;
  for (const line of lines) {
    const m = line.match(re);
    if (!m) continue;
    const wpId = m[1].trim();
    const token = m[2].trim();
    if (!wpId || !token) continue;
    entries.push({ wp_id: wpId, status_token: token });
  }

  entries.sort((a, b) => {
    const byId = compareStrings(a.wp_id, b.wp_id);
    if (byId !== 0) return byId;
    return compareStrings(a.status_token, b.status_token);
  });

  return entries;
};

const parseTraceabilityMappings = (traceabilityText) => {
  const mappings = [];
  const lines = (traceabilityText ?? '').split('\n');
  for (const rawLine of lines) {
    const line = rawLine.trimEnd();
    if (!line.trimStart().startsWith('|')) continue;
    if (line.includes('Base WP ID') || line.includes('---')) continue;
    const cols = line.split('|').slice(1, -1).map((c) => c.trim());
    if (cols.length < 2) continue;
    const baseWpId = cols[0];
    const activePacketPath = cols[1];
    if (!baseWpId || !activePacketPath) continue;
    mappings.push({ base_wp_id: baseWpId, active_packet_path: activePacketPath });
  }

  mappings.sort((a, b) => compareStrings(a.base_wp_id, b.base_wp_id));
  return mappings;
};

const parseSignatureAudit = (signatureAuditText) => {
  const consumed = [];
  const lines = (signatureAuditText ?? '').split('\n');
  for (const rawLine of lines) {
    const line = rawLine.trimEnd();
    if (!line.trimStart().startsWith('|')) continue;
    if (line.includes('| Signature |')) continue;
    if (line.includes('|-----------')) continue;
    const cols = line.split('|').slice(1, -1).map((c) => c.trim());
    if (cols.length < 4) continue;
    const signature = cols[0];
    const purpose = cols[3];
    if (!signature || !purpose) continue;

    const wpMatch = purpose.match(/\bWP-[A-Za-z0-9][A-Za-z0-9-]*\b/);
    const row = { signature, purpose };
    if (wpMatch?.[0]) row.wp_id = wpMatch[0];
    consumed.push(row);
  }

  consumed.sort((a, b) => compareStrings(a.signature, b.signature));
  return consumed;
};

const summarizeOrchestratorGates = (gatesJsonText) => {
  let parsed;
  try {
    parsed = JSON.parse(gatesJsonText);
  } catch {
    throw new Error('ORCHESTRATOR_GATES_INVALID_JSON');
  }

  const logs = Array.isArray(parsed.gate_logs) ? parsed.gate_logs : [];
  const findLast = (type) => {
    for (let i = logs.length - 1; i >= 0; i -= 1) {
      const entry = logs[i];
      if (entry && entry.type === type) return entry;
    }
    return null;
  };

  const lastRefinement = findLast('REFINEMENT');
  const lastSignature = findLast('SIGNATURE');
  const lastPrepare = findLast('PREPARE');

  const orchestrator = {};
  if (lastRefinement?.wpId) orchestrator.last_refinement = String(lastRefinement.wpId);
  if (lastSignature?.signature) orchestrator.last_signature = String(lastSignature.signature);
  if (lastPrepare?.wpId) orchestrator.last_prepare = String(lastPrepare.wpId);

  return orchestrator;
};

const summarizeValidatorGates = (validatorGateJsonByPath) => {
  const summaries = [];

  for (const [relPath, jsonText] of validatorGateJsonByPath.entries()) {
    let parsed;
    try {
      parsed = JSON.parse(jsonText);
    } catch {
      throw new Error(`VALIDATOR_GATES_INVALID_JSON: ${relPath}`);
    }

    const sessions = parsed && typeof parsed === 'object' && parsed.validation_sessions && typeof parsed.validation_sessions === 'object'
      ? parsed.validation_sessions
      : {};

    const keys = Object.keys(sessions).sort(compareStrings);
    for (const k of keys) {
      const session = sessions[k] ?? {};
      const wpId = typeof session.wpId === 'string' && session.wpId.trim() ? session.wpId.trim() : k;
      const summary = { wp_id: wpId };

      if (typeof session.verdict === 'string' && session.verdict.trim()) summary.verdict = session.verdict.trim();
      if (typeof session.status === 'string' && session.status.trim()) summary.status = session.status.trim();

      if (Array.isArray(session.gates)) {
        const gates = session.gates
          .map((g) => (g && typeof g.gate === 'string' ? g.gate.trim() : ''))
          .filter(Boolean);
        const unique = Array.from(new Set(gates)).sort(compareStrings);
        if (unique.length > 0) summary.gates_passed = unique;
      }

      summaries.push(summary);
    }
  }

  summaries.sort((a, b) => compareStrings(a.wp_id, b.wp_id));
  return summaries;
};

const getHeadSha = () => {
  try {
    const out = execSync('git rev-parse HEAD', { cwd: REPO_ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
    return /^[a-f0-9]{40}$/i.test(out) ? out.toLowerCase() : null;
  } catch {
    return null;
  }
};

export const buildProductGovernanceSnapshot = ({ includeHeadSha = false } = {}) => {
  const { inputsRelPaths, specFileRelPath } = computeWhitelistedInputRelPaths();
  const allowedAbs = new Set(inputsRelPaths.map((rel) => absFromRel(rel)));

  const fileBufCache = new Map();
  const readBufferStrict = (rel) => {
    const relNorm = normalizeRelPath(rel);
    const abs = absFromRel(relNorm);
    if (!allowedAbs.has(abs)) {
      throw new Error(`WHITELIST_VIOLATION: attempted read of non-whitelisted path: ${relNorm}`);
    }
    if (fileBufCache.has(relNorm)) return fileBufCache.get(relNorm);
    const buf = fs.readFileSync(abs);
    fileBufCache.set(relNorm, buf);
    return buf;
  };

  const readTextStrict = (rel) => readBufferStrict(rel).toString('utf8');
  const sha256Strict = (rel) => shaHex('sha256', readBufferStrict(rel));

  const specSha1 = shaHex('sha1', readBufferStrict(specFileRelPath));

  const taskBoardText = readTextStrict('.GOV/roles_shared/TASK_BOARD.md');
  const traceabilityText = readTextStrict('.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md');
  const signatureAuditText = readTextStrict('.GOV/roles_shared/SIGNATURE_AUDIT.md');
  const orchestratorGatesText = readTextStrict('.GOV/roles/orchestrator/ORCHESTRATOR_GATES.json');

  const taskBoardEntries = parseTaskBoardEntries(taskBoardText);
  const traceabilityMappings = parseTraceabilityMappings(traceabilityText);
  const consumedSignatures = parseSignatureAudit(signatureAuditText);

  const orchestratorGateSummary = summarizeOrchestratorGates(orchestratorGatesText);

  const validatorGatePaths = inputsRelPaths
    .filter((p) => p.startsWith('.GOV/validator_gates/') && p.toLowerCase().endsWith('.json'))
    .map(normalizeRelPath)
    .sort(compareStrings);

  const validatorGateJsonByPath = new Map();
  for (const rel of validatorGatePaths) {
    validatorGateJsonByPath.set(rel, readTextStrict(rel));
  }
  const wpGateSummaries = summarizeValidatorGates(validatorGateJsonByPath);

  const inputs = inputsRelPaths
    .map((rel) => {
      const relNorm = normalizeRelPath(rel);
      return { path: relNorm, sha256: sha256Strict(relNorm) };
    })
    .sort((a, b) => compareStrings(a.path, b.path));

  const git = {};
  if (includeHeadSha) {
    const headSha = getHeadSha();
    if (headSha) git.head_sha = headSha;
  }

  const snapshot = {
    schema_version: SCHEMA_VERSION,
    spec: {
      spec_target: normalizeRelPath(specFileRelPath),
      spec_sha1: specSha1,
    },
    git,
    inputs,
    task_board: {
      entries: taskBoardEntries,
    },
    traceability: {
      mappings: traceabilityMappings,
    },
    signatures: {
      consumed: consumedSignatures,
    },
    gates: {
      orchestrator: orchestratorGateSummary,
      validator: wpGateSummaries.length > 0 ? { wp_gate_summaries: wpGateSummaries } : {},
    },
  };

  return { snapshot, inputsRelPaths };
};

export const writeProductGovernanceSnapshot = ({ outPath = DEFAULT_OUTPUT_PATH, includeHeadSha = false } = {}) => {
  const outRel = normalizeRelPath(outPath);
  const outAbs = absFromRel(outRel);

  const parent = path.dirname(outAbs);
  if (!fs.existsSync(parent)) {
    throw new Error(`OUTPUT_DIR_MISSING: ${toPosixRelPath(path.relative(REPO_ROOT, parent)) || parent}`);
  }

  const { snapshot } = buildProductGovernanceSnapshot({ includeHeadSha });
  const bytes = `${JSON.stringify(snapshot, null, 2)}\n`;

  fs.writeFileSync(outAbs, bytes, 'utf8');
  return { outRelPath: outRel, bytes, snapshot };
};

const usage = () => [
  'Usage: node .GOV/scripts/governance-snapshot.mjs [--out <path>] [--include-head-sha]',
  '',
  'Defaults:',
  `  --out ${DEFAULT_OUTPUT_PATH}`,
  '  (git.head_sha omitted unless --include-head-sha is set)',
].join('\n');

const main = () => {
  const argv = process.argv.slice(2);
  if (argv.includes('-h') || argv.includes('--help')) {
    console.log(usage());
    process.exit(0);
  }

  let outPath = DEFAULT_OUTPUT_PATH;
  let includeHeadSha = false;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--out') {
      const next = argv[i + 1];
      if (!next) {
        console.error('ERROR: --out requires a value.\n');
        console.error(usage());
        process.exit(1);
      }
      outPath = next;
      i += 1;
      continue;
    }
    if (arg === '--include-head-sha') {
      includeHeadSha = true;
      continue;
    }
    console.error(`ERROR: Unknown argument: ${arg}\n`);
    console.error(usage());
    process.exit(1);
  }

  try {
    const res = writeProductGovernanceSnapshot({ outPath, includeHeadSha });
    console.log(`Wrote: ${res.outRelPath}`);
  } catch (err) {
    console.error(String(err?.message || err));
    process.exit(1);
  }
};

const isDirect = path.resolve(process.argv[1] || '') === path.resolve(SCRIPT_PATH);
if (isDirect) main();

