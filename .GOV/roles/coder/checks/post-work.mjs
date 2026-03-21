#!/usr/bin/env node
/**
 * `just post-work WP-{ID} ...` wrapper (chat-ready) [CX-GATE-UX-001]
 *
 * Goals:
 * - Keep correctness: run the same underlying gates as before.
 * - Reduce babysitting: emit chat-ready blocks automatically.
 *
 * What this runs (in order):
 * 1) Phase gate: gate-check.mjs [CX-GATE-001]
 * 2) Deterministic manifest validation: post-work-check.mjs [CX-623, CX-651]
 * 3) Role mailbox export gate: role_mailbox_export_check.mjs
 */

import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { GOV_ROOT_REPO_REL } from '../../../roles_shared/scripts/lib/runtime-paths.mjs';

const wpId = process.argv[2];
const extraArgs = process.argv.slice(3);
if (!wpId) {
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles/coder/checks/post-work.mjs WP-{ID} [options]`);
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

let ok = true;
let why = 'Post-work checks passed.';

printBlockHeader('GATE_OUTPUT', 'CX-GATE-UX-001');
process.stdout.write('\n');

const gateCheckPath = path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'checks', 'gate-check.mjs');
const postWorkCheckPath = path.join(GOV_ROOT_REPO_REL, 'roles', 'coder', 'checks', 'post-work-check.mjs');
const roleMailboxPath = path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'checks', 'role_mailbox_export_check.mjs');
const communicationHealthPath = path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'checks', 'wp-communication-health-check.mjs');
let communicationHealthOk = true;

const gate = run(process.execPath, [gateCheckPath, wpId]);
process.stdout.write(ensureTrailingNewline(gate.out.trimEnd()));
if (gate.code !== 0) {
  ok = false;
  why = 'Phase gate failed.';
}

process.stdout.write('\n');

const post = run(process.execPath, [postWorkCheckPath, wpId, ...extraArgs]);
process.stdout.write(ensureTrailingNewline(post.out.trimEnd()));
if (post.code !== 0) {
  ok = false;
  why = 'Deterministic post-work validation failed.';
}

process.stdout.write('\n');

const roleMailbox = run(process.execPath, [roleMailboxPath]);
process.stdout.write(ensureTrailingNewline(roleMailbox.out.trimEnd()));
if (roleMailbox.code !== 0) {
  ok = false;
  why = 'Role mailbox export gate failed.';
}

process.stdout.write('\n');

const communicationHealth = run(process.execPath, [communicationHealthPath, wpId, 'KICKOFF']);
process.stdout.write(ensureTrailingNewline(communicationHealth.out.trimEnd()));
if (communicationHealth.code !== 0) {
  ok = false;
  communicationHealthOk = false;
  if (why === 'Post-work checks passed.') {
    why = 'Direct review communication contract is not satisfied.';
  }
}

process.stdout.write('\n');

printBlockHeader('GATE_STATUS', 'CX-GATE-UX-001');
process.stdout.write(`- PHASE: POST_WORK\n`);
process.stdout.write(`- GATE_RAN: just post-work ${wpId}${extraArgs.length ? ' ' + extraArgs.join(' ') : ''}\n`);
process.stdout.write(`- RESULT: ${ok ? 'PASS' : 'FAIL'}\n`);
process.stdout.write(`- WHY: ${why}\n`);

process.stdout.write('\n');
printBlockHeader('NEXT_COMMANDS', 'CX-GATE-UX-001');
if (ok) {
  process.stdout.write(`- You may proceed with commit.\n`);
  process.stdout.write(`- Record the structured coder handoff before notifying the validator: just wp-coder-handoff ${wpId} <coder-session> <validator-session> "<summary>"\n`);
} else {
  process.stdout.write(`- Review the failures above.\n`);
  if (!communicationHealthOk) {
    process.stdout.write(`- Complete the direct review contract, then re-run: just wp-communication-health-check ${wpId} KICKOFF\n`);
  }
  process.stdout.write(`- Fix issues, then re-run: just post-work ${wpId}\n`);
}

process.exit(ok ? 0 : 1);

