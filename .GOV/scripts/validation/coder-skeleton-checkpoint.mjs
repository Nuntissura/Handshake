#!/usr/bin/env node
/**
 * Helper: docs-only skeleton checkpoint commit for a WP [CX-GATE-001, CX-625]
 *
 * Enforces:
 * - Only `.GOV/task_packets/{WP_ID}.md` is modified/staged/untracked.
 * - Creates a commit: "docs: skeleton checkpoint [WP-{ID}]"
 *
 * This is the interface-first checkpoint required before implementation [CX-GATE-001].
 */

import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';

const wpId = process.argv[2];
if (!wpId) {
  console.error('Usage: node .GOV/scripts/validation/coder-skeleton-checkpoint.mjs WP-{ID}');
  process.exit(1);
}

const packetRel = path.join('.GOV', 'task_packets', `${wpId}.md`);
if (!fs.existsSync(packetRel)) {
  console.error(`FAIL: Task packet not found: ${packetRel}`);
  process.exit(1);
}

function git(args) {
  return execSync(`git ${args}`, { encoding: 'utf8' });
}

const checkpointSubject = `docs: skeleton checkpoint [${wpId}]`;

function latestCheckpointSha() {
  const log = git('log -n 50 --format=%H%x09%s');
  for (const line of log.split(/\r?\n/)) {
    if (!line) continue;
    const [sha, subject] = line.split('\t');
    if ((subject || '').trim() === checkpointSubject) {
      return (sha || '').trim();
    }
  }
  return '';
}

const currentBranch = git('rev-parse --abbrev-ref HEAD').trim();

// Refuse obvious wrong contexts.
if (currentBranch === 'main' || currentBranch.startsWith('role_') || currentBranch.startsWith('user_')) {
  console.error(`FAIL: Refusing to create a WP skeleton commit on branch "${currentBranch}".`);
  console.error(`Expected a WP feature branch for ${wpId}.`);
  process.exit(1);
}

const status = git('status --porcelain=v1');
const lines = status.split(/\r?\n/).filter(Boolean);

// Collect paths (including untracked) and enforce docs-only.
const changed = lines.map((l) => {
  const m = l.match(/^\s*\S\S\s+(.+)\s*$/);
  return m ? m[1].trim() : '';
}).filter(Boolean);

const allowed = new Set([packetRel.replace(/\\/g, '/')]);
const bad = changed.filter((p) => !allowed.has(p.replace(/\\/g, '/')));

if (bad.length > 0) {
  console.error('FAIL: Skeleton checkpoint must be docs-only.');
  console.error(`Allowed: ${packetRel.replace(/\\/g, '/')}`);
  console.error('Found additional changed/untracked paths:');
  bad.forEach((p) => console.error(`- ${p}`));
  process.exit(1);
}

if (changed.length === 0) {
  const checkpointSha = latestCheckpointSha();
  if (checkpointSha) {
    console.log(`PASS: Skeleton checkpoint already exists at ${checkpointSha}. Nothing new to commit.`);
    process.exit(0);
  }
  console.error(`FAIL: No changes detected. Edit ${packetRel.replace(/\\/g, '/')} (## SKELETON), then re-run.`);
  process.exit(1);
}

// Stage and commit.
execSync(`git add ${packetRel}`, { stdio: 'inherit' });
execSync(`git commit -m \"${checkpointSubject}\"`, { stdio: 'inherit' });
