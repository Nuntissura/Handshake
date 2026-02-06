#!/usr/bin/env node
/**
 * Product Governance Snapshot validator (HARD)
 * Spec anchor: Handshake_Master_Spec_v02.125.md 7.5.4.10
 *
 * Requirements:
 * - Generate twice and byte-compare
 * - Enforce whitelist-only inputs (as reflected in snapshot.inputs)
 * - Validate minimum schema + stable sorting
 * - Enforce no timestamps/raw logs in output
 * - Exit nonzero on any mismatch
 */

import path from 'path';
import { fileURLToPath } from 'url';
import {
  SCHEMA_VERSION,
  DEFAULT_OUTPUT_PATH,
  computeWhitelistedInputRelPaths,
  writeProductGovernanceSnapshot,
} from '../governance-snapshot.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCRIPT_DIR = path.dirname(SCRIPT_PATH);
const REPO_ROOT = path.resolve(SCRIPT_DIR, '..', '..', '..');

const compareStrings = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

const isPlainObject = (v) => !!v && typeof v === 'object' && !Array.isArray(v);

const fail = (msg) => {
  console.error(`ERROR: ${msg}`);
  process.exit(1);
};

const assertSorted = (arr, keyFn, label) => {
  for (let i = 1; i < arr.length; i += 1) {
    const prev = keyFn(arr[i - 1]);
    const cur = keyFn(arr[i]);
    if (compareStrings(prev, cur) > 0) {
      fail(`${label} not sorted ascending (idx=${i} prev=${JSON.stringify(prev)} cur=${JSON.stringify(cur)})`);
    }
  }
};

const assertNoTimestampLikeText = (bytes) => {
  const patterns = [
    /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/, // RFC3339-ish
    /\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}/, // "YYYY-MM-DD HH:MM"
    /"timestamp"\s*:/i,
    /"started"\s*:/i,
    /"completed"\s*:/i,
    /"recorded_at"\s*:/i,
    /"turn_token"\s*:/i,
  ];
  for (const re of patterns) {
    if (re.test(bytes)) {
      fail(`timestamp/raw-log invariant violated (pattern ${re.toString()})`);
    }
  }
};

const assertNoForbiddenKeys = (obj) => {
  const forbiddenKeyRe = /^(timestamp|started|completed|recorded_at|turn_token)$/i;
  const stack = [obj];
  while (stack.length > 0) {
    const cur = stack.pop();
    if (Array.isArray(cur)) {
      cur.forEach((x) => stack.push(x));
      continue;
    }
    if (!isPlainObject(cur)) continue;
    Object.keys(cur).forEach((k) => {
      if (forbiddenKeyRe.test(k)) fail(`forbidden key present in snapshot: ${k}`);
      stack.push(cur[k]);
    });
  }
};

const validateSchema = (snapshot) => {
  if (!isPlainObject(snapshot)) fail('snapshot must be an object');
  if (snapshot.schema_version !== SCHEMA_VERSION) fail(`schema_version mismatch: expected ${SCHEMA_VERSION}`);

  // spec
  if (!isPlainObject(snapshot.spec)) fail('spec must be an object');
  if (typeof snapshot.spec.spec_target !== 'string' || !snapshot.spec.spec_target.trim()) fail('spec.spec_target must be a non-empty string');
  if (typeof snapshot.spec.spec_sha1 !== 'string' || !/^[a-f0-9]{40}$/i.test(snapshot.spec.spec_sha1)) fail('spec.spec_sha1 must be a 40-char hex sha1');

  // git provenance
  if (!isPlainObject(snapshot.git)) fail('git must be an object');
  if (Object.prototype.hasOwnProperty.call(snapshot.git, 'head_sha')) {
    fail('git.head_sha must be omitted by default (only allowed behind explicit flag)');
  }

  // inputs
  if (!Array.isArray(snapshot.inputs)) fail('inputs must be a list');
  snapshot.inputs.forEach((it, idx) => {
    if (!isPlainObject(it)) fail(`inputs[${idx}] must be an object`);
    if (typeof it.path !== 'string' || !it.path.trim()) fail(`inputs[${idx}].path must be a string`);
    if (typeof it.sha256 !== 'string' || !/^[a-f0-9]{64}$/i.test(it.sha256)) fail(`inputs[${idx}].sha256 must be a 64-char hex sha256`);
  });
  assertSorted(snapshot.inputs, (x) => x.path, 'inputs');

  // task_board
  if (!isPlainObject(snapshot.task_board)) fail('task_board must be an object');
  if (!Array.isArray(snapshot.task_board.entries)) fail('task_board.entries must be a list');
  snapshot.task_board.entries.forEach((e, idx) => {
    if (!isPlainObject(e)) fail(`task_board.entries[${idx}] must be an object`);
    if (typeof e.wp_id !== 'string' || !e.wp_id.trim()) fail(`task_board.entries[${idx}].wp_id must be a string`);
    if (typeof e.status_token !== 'string' || !e.status_token.trim()) fail(`task_board.entries[${idx}].status_token must be a string`);
  });
  assertSorted(snapshot.task_board.entries, (x) => `${x.wp_id}\u0000${x.status_token}`, 'task_board.entries');

  // traceability
  if (!isPlainObject(snapshot.traceability)) fail('traceability must be an object');
  if (!Array.isArray(snapshot.traceability.mappings)) fail('traceability.mappings must be a list');
  snapshot.traceability.mappings.forEach((m, idx) => {
    if (!isPlainObject(m)) fail(`traceability.mappings[${idx}] must be an object`);
    if (typeof m.base_wp_id !== 'string' || !m.base_wp_id.trim()) fail(`traceability.mappings[${idx}].base_wp_id must be a string`);
    if (typeof m.active_packet_path !== 'string' || !m.active_packet_path.trim()) fail(`traceability.mappings[${idx}].active_packet_path must be a string`);
  });
  assertSorted(snapshot.traceability.mappings, (x) => x.base_wp_id, 'traceability.mappings');

  // signatures
  if (!isPlainObject(snapshot.signatures)) fail('signatures must be an object');
  if (!Array.isArray(snapshot.signatures.consumed)) fail('signatures.consumed must be a list');
  snapshot.signatures.consumed.forEach((s, idx) => {
    if (!isPlainObject(s)) fail(`signatures.consumed[${idx}] must be an object`);
    if (typeof s.signature !== 'string' || !s.signature.trim()) fail(`signatures.consumed[${idx}].signature must be a string`);
    if (typeof s.purpose !== 'string' || !s.purpose.trim()) fail(`signatures.consumed[${idx}].purpose must be a string`);
    if (Object.prototype.hasOwnProperty.call(s, 'wp_id') && (typeof s.wp_id !== 'string' || !s.wp_id.trim())) {
      fail(`signatures.consumed[${idx}].wp_id must be a non-empty string if present`);
    }
  });
  assertSorted(snapshot.signatures.consumed, (x) => x.signature, 'signatures.consumed');

  // gates
  if (!isPlainObject(snapshot.gates)) fail('gates must be an object');
  if (!isPlainObject(snapshot.gates.orchestrator)) fail('gates.orchestrator must be an object');
  if (!isPlainObject(snapshot.gates.validator)) fail('gates.validator must be an object');

  const orchKeys = Object.keys(snapshot.gates.orchestrator);
  const allowedOrchKeys = new Set(['last_refinement', 'last_signature', 'last_prepare']);
  orchKeys.forEach((k) => {
    if (!allowedOrchKeys.has(k)) fail(`gates.orchestrator has unexpected key: ${k}`);
    if (typeof snapshot.gates.orchestrator[k] !== 'string' || !snapshot.gates.orchestrator[k].trim()) fail(`gates.orchestrator.${k} must be a string`);
  });

  if (Object.prototype.hasOwnProperty.call(snapshot.gates.validator, 'wp_gate_summaries')) {
    if (!Array.isArray(snapshot.gates.validator.wp_gate_summaries)) fail('gates.validator.wp_gate_summaries must be a list');
    snapshot.gates.validator.wp_gate_summaries.forEach((s, idx) => {
      if (!isPlainObject(s)) fail(`wp_gate_summaries[${idx}] must be an object`);
      if (typeof s.wp_id !== 'string' || !s.wp_id.trim()) fail(`wp_gate_summaries[${idx}].wp_id must be a string`);
      if (Object.prototype.hasOwnProperty.call(s, 'verdict') && typeof s.verdict !== 'string') fail(`wp_gate_summaries[${idx}].verdict must be a string if present`);
      if (Object.prototype.hasOwnProperty.call(s, 'status') && typeof s.status !== 'string') fail(`wp_gate_summaries[${idx}].status must be a string if present`);
      if (Object.prototype.hasOwnProperty.call(s, 'gates_passed')) {
        if (!Array.isArray(s.gates_passed)) fail(`wp_gate_summaries[${idx}].gates_passed must be a list if present`);
        s.gates_passed.forEach((g, gIdx) => {
          if (typeof g !== 'string' || !g.trim()) fail(`wp_gate_summaries[${idx}].gates_passed[${gIdx}] must be a non-empty string`);
        });
        assertSorted(s.gates_passed, (x) => x, `wp_gate_summaries[${idx}].gates_passed`);
      }

      const allowed = new Set(['wp_id', 'verdict', 'status', 'gates_passed']);
      Object.keys(s).forEach((k) => {
        if (!allowed.has(k)) fail(`wp_gate_summaries[${idx}] has unexpected key: ${k}`);
      });
    });
    assertSorted(snapshot.gates.validator.wp_gate_summaries, (x) => x.wp_id, 'wp_gate_summaries');
  }
};

const validateWhitelist = (snapshot) => {
  const expected = computeWhitelistedInputRelPaths().inputsRelPaths
    .map((p) => p.replace(/\\/g, '/'))
    .sort(compareStrings);
  const actual = snapshot.inputs.map((i) => String(i.path)).sort(compareStrings);

  const expectedSet = new Set(expected);
  const actualSet = new Set(actual);

  const extra = actual.filter((p) => !expectedSet.has(p));
  const missing = expected.filter((p) => !actualSet.has(p));

  if (extra.length > 0) fail(`inputs include non-whitelisted paths: ${extra.join(', ')}`);
  if (missing.length > 0) fail(`inputs missing required whitelisted paths: ${missing.join(', ')}`);
};

const main = () => {
  if (process.argv.slice(2).includes('-h') || process.argv.slice(2).includes('--help')) {
    console.log('Usage: node .GOV/scripts/validation/validator-governance-snapshot.mjs');
    process.exit(0);
  }

  let first;
  let second;

  try {
    first = writeProductGovernanceSnapshot({ outPath: DEFAULT_OUTPUT_PATH, includeHeadSha: false });
    second = writeProductGovernanceSnapshot({ outPath: DEFAULT_OUTPUT_PATH, includeHeadSha: false });
  } catch (err) {
    fail(String(err?.message || err));
  }

  if (first.bytes !== second.bytes) {
    fail('nondeterministic output: second generation bytes differ from first');
  }

  const bytes = first.bytes;
  if (!bytes.endsWith('\n')) fail('output must end with a single LF newline');
  if (bytes.includes('\r')) fail('output must use LF newlines only (no CR characters)');

  assertNoTimestampLikeText(bytes);

  let snapshot;
  try {
    snapshot = JSON.parse(bytes);
  } catch {
    fail('output is not valid JSON');
  }

  assertNoForbiddenKeys(snapshot);
  validateSchema(snapshot);
  validateWhitelist(snapshot);

  // Success (no verdict claims in repo artifacts; exit code is the gate).
  console.log(`OK: ${DEFAULT_OUTPUT_PATH}`);
  console.log(`OK: repo_root=${REPO_ROOT.replace(/\\\\/g, '/')}`);
};

const isDirect = path.resolve(process.argv[1] || '') === path.resolve(SCRIPT_PATH);
if (isDirect) main();

