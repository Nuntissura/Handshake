#!/usr/bin/env node
/**
 * Helper: workflow-authority skeleton approval commit for a WP [CX-GATE-001]
 *
 * Purpose:
 * - After the Coder produces the BOOTSTRAP + SKELETON and creates a docs-only skeleton checkpoint commit,
 *   the workflow hard-gates until the workflow authority approves the skeleton.
 *
 * Enforcement:
 * - Refuses to run unless invoked from `role_validator`, a `user_*` branch,
 *   or `role_orchestrator` when the packet declares `WORKFLOW_LANE: ORCHESTRATOR_MANAGED`.
 * - Locates the WP worktree for `feat/{WP_ID}` and creates an allow-empty commit:
 *     "docs: skeleton approved [WP-{ID}]"
 */

import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { GOV_ROOT_REPO_REL } from '../scripts/lib/runtime-paths.mjs';

const wpId = process.argv[2];
if (!wpId) {
  console.error(`Usage: node ${GOV_ROOT_REPO_REL}/roles_shared/checks/skeleton-approved.mjs WP-{ID}`);
  process.exit(1);
}

function git(args, { cwd } = {}) {
  return execSync(`git ${args}`, { encoding: 'utf8', cwd });
}

function gitTrim(args, opts) {
  return (git(args, opts) || '').trim();
}

function die(msg) {
  console.error(msg);
  process.exit(1);
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, 'mi');
  const m = text.match(re);
  return m ? m[1].trim() : '';
}

const packetPath = path.join(GOV_ROOT_REPO_REL, 'task_packets', `${wpId}.md`);
let workflowLane = '';
if (fs.existsSync(packetPath)) {
  try {
    workflowLane = parseSingleField(fs.readFileSync(packetPath, 'utf8'), 'WORKFLOW_LANE').toUpperCase();
  } catch {
    workflowLane = '';
  }
}

const actorBranch = gitTrim('rev-parse --abbrev-ref HEAD');
const actorAllowed =
  actorBranch === 'role_validator' ||
  actorBranch === 'main' ||
  actorBranch.startsWith('feat/') ||
  actorBranch.startsWith('user_') ||
  (actorBranch === 'role_orchestrator' && workflowLane === 'ORCHESTRATOR_MANAGED');
if (!actorAllowed) {
  const approverHint =
    workflowLane === 'ORCHESTRATOR_MANAGED'
      ? 'Orchestrator/Operator/Validator'
      : 'Operator/Validator';
  die(
    `FAIL: Refusing skeleton approval from branch "${actorBranch}".\n` +
      `This command is restricted to ${approverHint}.\n` +
      `Ask ${approverHint} to run: just skeleton-approved ${wpId}`,
  );
}

const wpBranch = `feat/${wpId}`;
try {
  execSync(`git show-ref --verify --quiet refs/heads/${wpBranch}`);
} catch {
  die(`FAIL: WP branch not found: ${wpBranch}`);
}

function parseWorktrees(porcelain) {
  const lines = porcelain.split(/\r?\n/);
  const entries = [];
  let cur = null;
  for (const line of lines) {
    if (line.startsWith('worktree ')) {
      if (cur) entries.push(cur);
      cur = { path: line.slice('worktree '.length).trim(), head: '', branch: '' };
      continue;
    }
    if (!cur) continue;
    if (line.startsWith('HEAD ')) cur.head = line.slice('HEAD '.length).trim();
    if (line.startsWith('branch ')) cur.branch = line.slice('branch '.length).trim();
  }
  if (cur) entries.push(cur);
  return entries;
}

const porcelain = gitTrim('worktree list --porcelain');
const worktrees = parseWorktrees(porcelain);
const targetRef = `refs/heads/${wpBranch}`;
const wpWorktree = worktrees.find((w) => w.branch === targetRef);

if (!wpWorktree?.path) {
  die(
    `FAIL: No worktree found for ${wpBranch}.\n` +
      `Expected to find a worktree entry with branch "${targetRef}".\n` +
      `Create it first, then re-run this command.`,
  );
}

const wpWorktreePath = wpWorktree.path;

const dirty = gitTrim('status --porcelain=v1', { cwd: wpWorktreePath });
if (dirty) {
  die(
    `FAIL: Refusing to approve skeleton while WP worktree is dirty: ${wpWorktreePath}\n` +
      `Ask the Coder to clean/commit/stash their changes, then re-run: just skeleton-approved ${wpId}`,
  );
}

const escapeRegex = (s) => (s ?? '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const checkpointRe = `^docs: skeleton checkpoint \\[${escapeRegex(wpId)}\\]$`;
const approvedRe = `^docs: skeleton approved \\[${escapeRegex(wpId)}\\]$`;

const checkpointSha = gitTrim(`log -n 1 --format=%H --grep="${checkpointRe}"`, { cwd: wpWorktreePath });
if (!checkpointSha) {
  die(
    `FAIL: No skeleton checkpoint commit found for ${wpId} on ${wpBranch}.\n` +
      `Expected commit subject: docs: skeleton checkpoint [${wpId}] (create via: just coder-skeleton-checkpoint ${wpId})`,
  );
}

const alreadyApprovedSha = gitTrim(`log -n 1 --format=%H --grep="${approvedRe}"`, { cwd: wpWorktreePath });
if (alreadyApprovedSha) {
  console.log('SKELETON_APPROVED [CX-GATE-001]');
  console.log(`- WP_ID: ${wpId}`);
  console.log(`- WP_BRANCH: ${wpBranch}`);
  console.log(`- WP_WORKTREE: ${wpWorktreePath}`);
  console.log(`- APPROVER_BRANCH: ${actorBranch}`);
  console.log(`- WORKFLOW_LANE: ${workflowLane || '<unknown>'}`);
  console.log(`- SKELETON_CHECKPOINT_SHA: ${checkpointSha}`);
  console.log(`- APPROVAL_COMMIT_SHA: ${alreadyApprovedSha}`);
  console.log(`- STATUS: ALREADY_APPROVED`);
  process.exit(0);
}

execSync(`git commit --allow-empty -m "docs: skeleton approved [${wpId}]"`, {
  stdio: 'inherit',
  cwd: wpWorktreePath,
});

const approvalSha = gitTrim('rev-parse HEAD', { cwd: wpWorktreePath });

console.log('\nSKELETON_APPROVED [CX-GATE-001]');
console.log(`- WP_ID: ${wpId}`);
console.log(`- WP_BRANCH: ${wpBranch}`);
console.log(`- WP_WORKTREE: ${wpWorktreePath}`);
console.log(`- APPROVER_BRANCH: ${actorBranch}`);
console.log(`- WORKFLOW_LANE: ${workflowLane || '<unknown>'}`);
console.log(`- SKELETON_CHECKPOINT_SHA: ${checkpointSha}`);
console.log(`- APPROVAL_COMMIT_SHA: ${approvalSha}`);
console.log(`- NEXT: Coder re-run 'just pre-work ${wpId}' then proceed to IMPLEMENTATION.`);
