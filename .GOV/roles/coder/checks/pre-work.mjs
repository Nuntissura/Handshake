#!/usr/bin/env node
/**
 * `just pre-work WP-{ID}` wrapper (chat-ready) [CX-GATE-UX-001]
 *
 * Goals:
 * - Keep correctness: run the same underlying gates as before.
 * - Reduce babysitting: emit chat-ready blocks automatically.
 *
 * What this runs (in order):
 * 1) Phase gate: gate-check.mjs [CX-GATE-001]
 * 2) Pre-work validation: pre-work-check.mjs [CX-580, CX-620]
 */

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { GOV_ROOT_REPO_REL, resolveWorkPacketPath } from '../../../roles_shared/scripts/lib/runtime-paths.mjs';

const wpId = process.argv[2];
if (!wpId) {
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles/coder/checks/pre-work.mjs WP-{ID}`);
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

function escapeRegex(s) {
  return (s ?? '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, 'mi');
  const m = text.match(re);
  return m ? m[1].trim() : '';
}

function workflowLaneForPacket(wpId) {
  const resolved = resolveWorkPacketPath(wpId);
  const packetPath = resolved?.packetPath || path.join(GOV_ROOT_REPO_REL, 'task_packets', `${wpId}.md`);
  try {
    const packetText = ensureTrailingNewline(fs.readFileSync(packetPath, 'utf8'));
    return parseSingleField(packetText, 'WORKFLOW_LANE').toUpperCase();
  } catch {
    return '';
  }
}

const gateOutputs = [];
let ok = true;
let why = 'Pre-work checks passed.';
let blockedOnSkeletonApproval = false;
let blockedOnBootstrapClaim = false;
const workflowLane = workflowLaneForPacket(wpId);
const skeletonApprover =
  workflowLane === 'ORCHESTRATOR_MANAGED' ? 'Orchestrator/Validator/Operator' : 'Operator/Validator';

printBlockHeader('GATE_OUTPUT', 'CX-GATE-UX-001');
process.stdout.write('\n');

const gateCheckPath = path.join(GOV_ROOT_REPO_REL, 'roles_shared', 'checks', 'gate-check.mjs');
const preWorkCheckPath = path.join(GOV_ROOT_REPO_REL, 'roles', 'coder', 'checks', 'pre-work-check.mjs');

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
  blockedOnBootstrapClaim = /Missing docs-only bootstrap claim commit/i.test(pre.out);
}

// Hard gate: after skeleton checkpoint exists, require Operator/Validator approval commit.
const checkpointSubjectRe = `^docs: skeleton checkpoint \\[${escapeRegex(wpId)}\\]$`;
const approvedSubjectRe = `^docs: skeleton approved \\[${escapeRegex(wpId)}\\]$`;

const hasCommitBySubjectRe = (subjectRe) => {
  const res = run('git', ['log', '-n', '1', '--format=%H', `--grep=${subjectRe}`]);
  if (res.code !== 0) return false;
  return Boolean((res.out || '').trim());
};

const hasSkeletonCheckpoint = hasCommitBySubjectRe(checkpointSubjectRe);
const hasSkeletonApproval = hasCommitBySubjectRe(approvedSubjectRe);

if (ok && hasSkeletonCheckpoint && !hasSkeletonApproval) {
  ok = false;
  blockedOnSkeletonApproval = true;
  why = `Skeleton checkpoint exists; awaiting ${skeletonApprover} approval.`;
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
  if (!hasSkeletonCheckpoint) {
    process.stdout.write(`- (After updating the packet \`## SKELETON\`) just coder-skeleton-checkpoint ${wpId}\n`);
    process.stdout.write(`- STOP: Await skeleton approval (${skeletonApprover} runs: just skeleton-approved ${wpId})\n`);
    process.stdout.write(`- After approval commit exists: re-run just pre-work ${wpId}\n`);
  } else {
    process.stdout.write(`- Proceed to implementation (skeleton approved).\n`);
  }
} else {
  if (blockedOnSkeletonApproval) {
    process.stdout.write(`- STOP: Await skeleton approval (${skeletonApprover} runs: just skeleton-approved ${wpId})\n`);
    process.stdout.write(`- After approval commit exists: re-run just pre-work ${wpId}\n`);
  } else if (blockedOnBootstrapClaim) {
    process.stdout.write(`- Create the required docs-only bootstrap claim commit: git commit -m "docs: bootstrap claim [${wpId}]"\n`);
    process.stdout.write(`- Re-run: just pre-work ${wpId}\n`);
  } else {
    process.stdout.write(`- Review the failures above.\n`);
    process.stdout.write(`- Fix the packet/worktree context, then re-run: just pre-work ${wpId}\n`);
  }
}

process.exit(ok ? 0 : 1);

