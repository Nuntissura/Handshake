#!/usr/bin/env node
/**
 * `just pre-work WP-{ID}` wrapper (chat-ready) [CX-GATE-UX-001]
 *
 * Goals:
 * - Keep correctness: run the same underlying gates as before.
 * - Reduce babysitting: emit chat-ready blocks automatically.
 * - Fold CX-WT-001 worktree evidence into pre-work output.
 *
 * What this runs (in order):
 * 1) Worktree evidence (git rev-parse/status/worktree list) [CX-WT-001]
 * 2) Phase gate: gate-check.mjs [CX-GATE-001]
 * 3) Pre-work validation: pre-work-check.mjs [CX-580, CX-620]
 */

import path from 'node:path';
import { spawnSync } from 'node:child_process';

const wpId = process.argv[2];
if (!wpId) {
  console.error('Usage: node .GOV/scripts/validation/pre-work.mjs WP-{ID}');
  process.exit(1);
}

function run(cmd, args) {
  const res = spawnSync(cmd, args, { encoding: 'utf8' });
  const out = `${res.stdout || ''}${res.stderr || ''}`;
  const code = typeof res.status === 'number' ? res.status : 1;
  return { code, out };
}

function printBlockHeader(name, cx) {
  process.stdout.write(`${name} [${cx}]\n`);
}

function ensureTrailingNewline(s) {
  if (!s) return '';
  return s.endsWith('\n') ? s : `${s}\n`;
}

const gateOutputs = [];
let ok = true;
let why = 'Pre-work checks passed.';

printBlockHeader('GATE_OUTPUT', 'CX-GATE-UX-001');
process.stdout.write('\n');

// Fold the worktree+branch evidence into the pre-work output.
printBlockHeader('HARD_GATE_OUTPUT', 'CX-WT-001');
try {
  const top = run('git', ['rev-parse', '--show-toplevel']);
  process.stdout.write(ensureTrailingNewline(top.out.trimEnd()));
  if (top.code !== 0) ok = false;

  const status = run('git', ['status', '-sb']);
  process.stdout.write(ensureTrailingNewline(status.out.trimEnd()));
  if (status.code !== 0) ok = false;

  const wtl = run('git', ['worktree', 'list']);
  process.stdout.write(ensureTrailingNewline(wtl.out.trimEnd()));
  if (wtl.code !== 0) ok = false;
} catch (e) {
  ok = false;
  process.stdout.write(`(error collecting worktree evidence: ${String(e)})\n`);
}

process.stdout.write('\n');

const gateCheckPath = path.join('.GOV', 'scripts', 'validation', 'gate-check.mjs');
const preWorkCheckPath = path.join('.GOV', 'scripts', 'validation', 'pre-work-check.mjs');

const gate = run(process.execPath, [gateCheckPath, wpId]);
gateOutputs.push(gate.out);
process.stdout.write(ensureTrailingNewline(gate.out.trimEnd()));
if (gate.code !== 0) {
  ok = false;
  why = 'Phase gate failed.';
}

process.stdout.write('\n');

const pre = run(process.execPath, [preWorkCheckPath, wpId]);
gateOutputs.push(pre.out);
process.stdout.write(ensureTrailingNewline(pre.out.trimEnd()));
if (pre.code !== 0) {
  ok = false;
  why = 'Pre-work validation failed.';
}

process.stdout.write('\n');

printBlockHeader('GATE_STATUS', 'CX-GATE-UX-001');
process.stdout.write(`- PHASE: BOOTSTRAP\n`);
process.stdout.write(`- GATE_RAN: just pre-work ${wpId}\n`);
process.stdout.write(`- RESULT: ${ok ? 'PASS' : 'FAIL'}\n`);
process.stdout.write(`- WHY: ${why}\n`);

process.stdout.write('\n');
printBlockHeader('NEXT_COMMANDS', 'CX-GATE-UX-001');
if (ok) {
  process.stdout.write(`- (After updating the packet \\`## SKELETON\\`) just coder-skeleton-checkpoint ${wpId}\n`);
  process.stdout.write(`- STOP and wait for \"SKELETON APPROVED\" before implementation.\n`);
} else {
  process.stdout.write(`- Review the failures above.\n`);
  process.stdout.write(`- Fix the packet/worktree context, then re-run: just pre-work ${wpId}\n`);
}

process.exit(ok ? 0 : 1);

